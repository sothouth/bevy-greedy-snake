use std::time::Duration;

use bevy::prelude::*;
use rand::{Rng, SeedableRng};

const BLOCK_SIZE: f32 = 20.0;
const BLOCK_DNUM: i32 = 5;
const RANGE: (f32, f32, f32, f32) = (
    -BLOCK_SIZE * BLOCK_DNUM as f32,
    BLOCK_SIZE * BLOCK_DNUM as f32,
    -BLOCK_SIZE * BLOCK_DNUM as f32,
    BLOCK_SIZE * BLOCK_DNUM as f32,
);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.5, 0.5, 0.5)))
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Greedy Snake".to_string(),
                    resolution: (
                        BLOCK_SIZE * BLOCK_DNUM as f32 * 2. + BLOCK_SIZE,
                        BLOCK_SIZE * BLOCK_DNUM as f32 * 2. + BLOCK_SIZE,
                    )
                        .into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
        )
        .insert_state(Motion::Stay)
        .add_systems(Startup, (setup, spawn_food).chain())
        .add_systems(
            PreUpdate,
            (
                check_control,
                spawn_food.run_if(in_state(Motion::Eat)),
                move_head,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (move_body, eat_self)
                .chain()
                .run_if(|s: Res<State<Motion>>| *s != Motion::Stay),
        )
        .run();
}

fn setup(mut cmd: Commands) {
    cmd.spawn(Camera2d);
    let start = Transform::from_xyz(0.0, 0.0, 0.0);
    let dir = Dir::Up;
    let head = (
        Head {
            pre: start.translation,
        },
        start,
        dir,
        SnakeTimer(Timer::from_seconds(1.0, TimerMode::Repeating)),
        block(Color::srgb(0.95, 0., 0.)),
    );
    cmd.spawn(head);
    let neck = (
        Body,
        Neck,
        Transform::from_translation(start.translation - dir),
        block(Color::WHITE),
    );
    let neck = cmd.spawn(neck).id();
    let tail = (
        Body,
        Tail,
        Transform::from_translation(start.translation - dir - dir),
        block(Color::WHITE),
        Pre(neck),
    );
    cmd.spawn(tail);

    let rander = rand::rngs::StdRng::from_os_rng();

    cmd.insert_resource(Rander(rander));
}

fn check_control(input: Res<ButtonInput<KeyCode>>, mut head_dir: Single<&mut Dir, With<Head>>) {
    if (input.just_pressed(KeyCode::KeyW) || input.just_pressed(KeyCode::ArrowUp))
        && **head_dir != Dir::Down
    {
        **head_dir = Dir::Up;
    } else if (input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown))
        && **head_dir != Dir::Up
    {
        **head_dir = Dir::Down;
    } else if (input.just_pressed(KeyCode::KeyA) || input.just_pressed(KeyCode::ArrowLeft))
        && **head_dir != Dir::Right
    {
        **head_dir = Dir::Left;
    } else if (input.just_pressed(KeyCode::KeyD) || input.just_pressed(KeyCode::ArrowRight))
        && **head_dir != Dir::Left
    {
        **head_dir = Dir::Right;
    }
}

fn spawn_food(
    mut cmd: Commands,
    taboos: Query<&Transform, Or<(With<Head>, With<Body>)>>,
    mut rander: ResMut<Rander>,
) {
    let admissable = (-BLOCK_DNUM..=BLOCK_DNUM)
        .flat_map(|x| {
            (-BLOCK_DNUM..=BLOCK_DNUM).map(move |y| (x as f32 * BLOCK_SIZE, y as f32 * BLOCK_SIZE))
        })
        .filter(|(x, y)| {
            taboos
                .iter()
                .all(|t| (t.translation.x - *x).abs() > 0.1 || (t.translation.y - *y).abs() > 0.1)
        })
        .collect::<Vec<_>>();
    let (x, y) = admissable[rander.0.random_range(0..admissable.len())];
    let transform = Transform::from_xyz(x, y, 0.0);
    cmd.spawn((Food, transform, block(Color::srgb(0., 0.95, 0.))));
}

fn move_head(
    mut cmd: Commands,
    time: Res<Time>,
    food: Query<(Entity, &Transform), (With<Food>, Without<Head>)>,
    head: Single<(&mut Transform, &Dir, &mut SnakeTimer, &mut Head)>,
    body: Query<(), With<Body>>,
    mut motion: ResMut<NextState<Motion>>,
) {
    let (mut head_pos, dir, mut elapsed, mut prehead) = head.into_inner();
    if !elapsed.0.tick(time.delta()).just_finished() {
        motion.set(Motion::Stay);
        return;
    }
    prehead.pre = head_pos.translation;
    head_pos.translation = {
        let mut new_pos = head_pos.translation + *dir;
        if new_pos.x - RANGE.0 < -0.1 {
            new_pos.x = RANGE.1;
        } else if new_pos.x - RANGE.1 > 0.1 {
            new_pos.x = RANGE.0;
        } else if new_pos.y - RANGE.2 < -0.1 {
            new_pos.y = RANGE.3;
        } else if new_pos.y - RANGE.3 > 0.1 {
            new_pos.y = RANGE.2;
        }
        new_pos
    };
    motion.set(Motion::Move);

    for (food, food_pos) in food.iter() {
        if head_pos.translation.distance(food_pos.translation) <= 1.0 {
            cmd.entity(food).despawn();
            elapsed.0.set_duration(fit_speed(body.iter().count()));
            motion.set(Motion::Eat);
        }
    }
}

fn move_body(
    mut cmd: Commands,
    eat: Res<State<Motion>>,
    head: Single<&Head>,
    neck: Single<Entity, With<Neck>>,
    tail: Single<(Entity, &mut Transform, &Pre), With<Tail>>,
) {
    let neck = neck.into_inner();
    let (tail, mut tail_pos, tail_pre) = tail.into_inner();
    if *eat == Motion::Eat {
        let new = cmd
            .spawn((
                Body,
                Neck,
                Transform::from_translation(head.pre),
                block(Color::WHITE),
            ))
            .id();
        cmd.entity(neck).remove::<Neck>();
        cmd.entity(neck).insert(Pre(new));
    } else {
        tail_pos.translation = head.pre;
        cmd.entity(tail).remove::<Tail>();
        cmd.entity(tail_pre.0).insert(Tail);
        cmd.entity(tail).remove::<Pre>();
        cmd.entity(neck).insert(Pre(tail));
        cmd.entity(neck).remove::<Neck>();
        cmd.entity(tail).insert(Neck);
    }
}

fn eat_self(
    // mut cmd: Commands,
    head: Single<&Transform, With<Head>>,
    body: Query<&Transform, With<Body>>,
) {
    for body in body.iter() {
        if head.translation.distance(body.translation) <= 1.0 {
            std::process::exit(0);
        }
    }
}

#[inline(always)]
fn block(color: Color) -> Sprite {
    Sprite::from_color(color, Vec2::new(BLOCK_SIZE, BLOCK_SIZE))
}

#[inline(always)]
fn fit_speed(len: usize) -> Duration {
    Duration::from_secs_f32(0.9 / (std::f32::consts::E.powf(((len - 1) as f32) / 40.0)) + 0.1)
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum Motion {
    Stay,
    Move,
    Eat,
}

#[derive(Resource)]
struct Rander(rand::rngs::StdRng);

#[derive(Component, Debug)]
struct SnakeTimer(Timer);

#[derive(Component, Debug)]
struct Pre(Entity);

#[derive(Component, Debug)]
struct Head {
    pre: Vec3,
}

#[derive(Component, Debug)]
struct Body;

#[derive(Component, Debug)]
struct Tail;

#[derive(Component, Debug)]
struct Neck;

#[derive(Component, Debug)]
struct Food;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl std::ops::Sub<Dir> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Dir) -> Self::Output {
        let mut res = self;
        match rhs {
            Dir::Up => res.y -= BLOCK_SIZE,
            Dir::Down => res.y += BLOCK_SIZE,
            Dir::Left => res.x += BLOCK_SIZE,
            Dir::Right => res.x -= BLOCK_SIZE,
        }
        res
    }
}

impl std::ops::Add<Dir> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Dir) -> Self::Output {
        let mut res = self;
        match rhs {
            Dir::Up => res.y += BLOCK_SIZE,
            Dir::Down => res.y -= BLOCK_SIZE,
            Dir::Left => res.x -= BLOCK_SIZE,
            Dir::Right => res.x += BLOCK_SIZE,
        }
        res
    }
}

impl std::ops::SubAssign<Dir> for Vec3 {
    fn sub_assign(&mut self, rhs: Dir) {
        match rhs {
            Dir::Up => self.y -= BLOCK_SIZE,
            Dir::Down => self.y += BLOCK_SIZE,
            Dir::Left => self.x += BLOCK_SIZE,
            Dir::Right => self.x -= BLOCK_SIZE,
        }
    }
}

impl std::ops::AddAssign<Dir> for Vec3 {
    fn add_assign(&mut self, rhs: Dir) {
        match rhs {
            Dir::Up => self.y += BLOCK_SIZE,
            Dir::Down => self.y -= BLOCK_SIZE,
            Dir::Left => self.x -= BLOCK_SIZE,
            Dir::Right => self.x += BLOCK_SIZE,
        }
    }
}
