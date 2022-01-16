use bevy::{diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, prelude::*};
use bevy_egui::{EguiContext, EguiPlugin, egui::Pos2};
use bevy_egui::egui;

enum PaddleType {
    Left,
    Right,
}

#[derive(Component)]
struct Paddle {
    paddle_type: PaddleType,
    is_auto: bool,
}

#[derive(Component)]
struct Position {
    y: f32,
}

#[derive(Component)]
struct Ball {
    x: f32,
    y: f32,
    speed_fact: f32,
}

#[derive(Component)]
struct Score {
    score: i64,
    paddle_type: PaddleType,
}

struct Counter {
    count: i64,
}

struct Sounds {
    ping: Handle<AudioSource>,
    button: Handle<AudioSource>,
}

/*
  We add a branch of functions (called system) to the engine.
  For `startup_system`, they will be executed only once at startup.
  For other systems, they will be executed in each frame.

  Each system fetches some data (through `Query`) and modify them.
*/
fn main() {
    App::new()
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .insert_resource(ClearColor(Color::rgb(1.0, 1.0, 1.0)))
        .insert_resource(Counter { count: 0 })
        .add_startup_system(setup)
        .add_startup_system(spawn_ball)
        .add_startup_system(spawn_paddle)
        .add_system(ball_move)
        .add_system(ball_speed_up)
        .add_system(transform_paddle)
        .add_system(move_paddle.system())
        .add_system(ball_collision.system())
        .add_system(auto_move_paddle.system())
        .add_system(ui.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d());
    commands.insert_resource(Sounds {
        button: asset_server.load("button.mp3"),
        ping: asset_server.load("ping.mp3"),
    });
}

/*
   render score
*/
fn ui(context: ResMut<EguiContext>, score: Query<&Score>, win: Res<Windows>,) {
    let win = win.get_primary().unwrap();
    egui::Window::new("score")
    .default_pos(Pos2::new(win.width()/2.0-20.0, 0.0))
    .hscroll(false)
    .show(context.ctx(), |ui| {
        for s in score.iter() {
            ui.label(match s.paddle_type {
                PaddleType::Left => format!("Left: {}", s.score),
                PaddleType::Right => format!("Right: {}", s.score)
            });
        }

        ui.label("1P(Left): move W/D, toggle auto P");
        ui.label("2P(Right): move Up/Down, toggle auto Q");
    });
}

/*
  Move the ball according to its speed.
  Direction, speed, etc. will be set in other system.
  We only focus on moving it here.
*/
fn ball_move(mut position: Query<(&Ball, &mut Transform)>) {
    for (ball, mut transform) in position.iter_mut() {
        transform.translation.x += ball.x * ball.speed_fact;
        transform.translation.y += ball.y * ball.speed_fact;
    }
}

/*
  Detect if the ball collapses into edge of screen.
  1) if it reaches top/bottom, we reverse its y.
  2) if it reaches left/right edge:
     a) a paddle catches it. we reverse its x.
     b) paddle fails to reach. This paddle(side) is lose.
*/
fn ball_collision(
    win: Res<Windows>,
    sounds: Res<Sounds>,
    audio: Res<Audio>,
    mut counter: ResMut<Counter>,
    mut position: Query<(&mut Ball, &mut Transform)>,
    paddle_position: Query<(&Paddle, &Position)>,
    mut scores: Query<&mut Score>,
) {
    let win = win.get_primary().unwrap();
    let height = win.height() as f32 / 2.0 - 20.;
    let width = win.width() as f32 / 2.0 - 20.;
    for (mut ball, mut transform) in position.iter_mut() {
        if transform.translation.y >= height || transform.translation.y <= -height {
            ball.y = -ball.y;
            audio.play(sounds.button.clone());
        }
        if transform.translation.x >= width {
            for (paddle, pos) in paddle_position.iter() {
                match paddle.paddle_type {
                    PaddleType::Right => {
                        if transform.translation.y > pos.y - 50.
                            && transform.translation.y < pos.y + 50.0
                        {
                            ball.x = -ball.x;
                            counter.count += 1;
                            audio.play(sounds.button.clone());
                            return;
                        }
                    }
                    _ => {}
                }
            }
            for mut score in scores.iter_mut() {
                match score.paddle_type {
                    PaddleType::Left => {
                        score.score += 1;
                    }
                    _ => {}
                }
            }
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            counter.count = 0;
            audio.play(sounds.ping.clone());
        } else if transform.translation.x <= -width {
            for (paddle, pos) in paddle_position.iter() {
                match paddle.paddle_type {
                    PaddleType::Left => {
                        if transform.translation.y > pos.y - 50.
                            && transform.translation.y < pos.y + 50.0
                        {
                            ball.x = -ball.x;
                            counter.count += 1;
                            audio.play(sounds.button.clone());
                            return;
                        }
                    }
                    _ => {}
                }
            }
            for mut score in scores.iter_mut() {
                match score.paddle_type {
                    PaddleType::Right => {
                        score.score += 1;
                    }
                    _ => {}
                }
            }
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            counter.count = 0;
            audio.play(sounds.ping.clone());
        }
    }
}

fn ball_speed_up(counter: Res<Counter>, mut ball: Query<&mut Ball>, score: Query<&Score>) {
    let cur_score = score.iter().map(|x| x.score).sum::<i64>();
    for mut ball in ball.iter_mut() {
        ball.speed_fact = 1.0 + (cur_score.min(20) as f32 / 5.) + (counter.count as f32 / 3.);
    }
}


/*
  move paddles
  Different from the ball, paddles have `Position` component as their y pos.
  `Position` will be modified in `*move_paddle`, here we just transform paddles.
  Of course, we should also make sure paddles only appears on left/right edges.
*/
fn transform_paddle(windows: Res<Windows>, mut q: Query<(&Paddle, &Position, &mut Transform)>) {
    let win = windows.get_primary().unwrap();
    let paddle_x = (win.width() / 2.0) - 10.0;
    for (paddle, pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            match paddle.paddle_type {
                PaddleType::Left => -paddle_x,
                PaddleType::Right => paddle_x,
            },
            pos.y,
            0.0,
        );
    }
}

/*
  automatically move paddles
*/
fn auto_move_paddle(
    win: Res<Windows>,
    mut q: Query<(&Paddle, &mut Position)>,
    b: Query<(&Ball, &Transform)>,
) {
    let win = win.get_primary().unwrap();
    let height = win.height() as f32;
    let width = win.width() as f32;
    let speed = height / 100.;

    let (ball, trans) = b.iter().next().unwrap();
    for (paddle, mut pos) in q.iter_mut() {
        if paddle.is_auto {
            let target_y = trans.translation.y
                + ball.y
                    * match paddle.paddle_type {
                        PaddleType::Left => (-width / 2.0 - trans.translation.x) / ball.x,
                        PaddleType::Right => (width / 2.0 - trans.translation.x) / ball.x,
                    };
            if target_y > pos.y {
                pos.y += speed;
            } else if target_y < pos.y {
                pos.y -= speed;
            }
        }
    }
}

/*
  move paddles according to `Input<KeyCode>`
  1) Left Paddle:
     W - up
     D - down
     P - auto/manual
  2) Right Paddle:
     UP Arrow - up
     Down Arrow - down
     Q - auto/manual
*/
fn move_paddle(
    input: Res<Input<KeyCode>>,
    win: Res<Windows>,
    mut q: Query<(&mut Paddle, &mut Position)>,
) {
    let height = win.get_primary().unwrap().height() as f32;
    let speed = height / 100.;
    for (mut paddle, mut pos) in q.iter_mut() {
        match paddle.paddle_type {
            PaddleType::Left => {
                if !paddle.is_auto {
                    if input.pressed(KeyCode::W) {
                        pos.y += speed;
                    }
                    if input.pressed(KeyCode::S) {
                        pos.y -= speed;
                    }
                }
                if input.just_pressed(KeyCode::Q) {
                    paddle.is_auto = !paddle.is_auto;
                }
            }
            PaddleType::Right => {
                if !paddle.is_auto {
                    if input.pressed(KeyCode::Up) {
                        pos.y += speed;
                    }
                    if input.pressed(KeyCode::Down) {
                        pos.y -= speed;
                    }
                }
                if input.just_pressed(KeyCode::P) {
                    paddle.is_auto = !paddle.is_auto;
                }
            }
        }
        pos.y = pos.y.min(height / 2.0 - 50.).max(-height / 2.0 + 50.);
    }
}

fn spawn_ball(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mat = asset_server.load("ball.png");
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            texture: mat,
            sprite: Sprite{
                custom_size: Some(Vec2::new(40.0, 40.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Ball {
            x: 3.0,
            y: 3.0,
            speed_fact: 1.0,
        });
}

/*
  Spawn two Paddles, two Score Textcomponents
*/
fn spawn_paddle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mat = asset_server.load("paddle1.png");
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            texture: mat.clone(),
            sprite: Sprite{
                custom_size: Some(Vec2::new(20.0, 100.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Paddle {
            paddle_type: PaddleType::Left,
            is_auto: true,
        })
        .insert(Position { y: 0.0 })
        .insert(Score {
            score: 0,
            paddle_type: PaddleType::Left,
        });
    commands.spawn()
        .insert_bundle(SpriteBundle {
            texture: mat,
            sprite: Sprite{
                custom_size: Some(Vec2::new(20.0, 100.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Position { y: 0.0 })
        .insert(Paddle {
            paddle_type: PaddleType::Right,
            is_auto: true,
        })
        .insert(Score {
            score: 0,
            paddle_type: PaddleType::Right,
        });
}
