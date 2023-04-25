use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

const TETRIS_HEIGHT: usize = 40;
const LEVEL_TIMES: [u32; 10] = [1000, 850, 700, 600, 500, 400, 300, 250, 221, 190];
const LEVEL_LINES: [u32; 10] = [20, 40, 60, 80, 100, 120, 140, 160, 180, 200];

fn create_texture_rect<'a>(
    canvas: &mut Canvas<Window>,
    texture_creator: &'a TextureCreator<WindowContext>,
    r: u8,
    g: u8,
    b: u8,
    width: u32,
    height: u32,
) -> Option<Texture<'a>> {
    if let Ok(mut square_texture) = texture_creator.create_texture_target(None, width, height) {
        canvas
            .with_texture_canvas(&mut square_texture, |texture| {
                texture.set_draw_color(Color::RGB(r, g, b));
                texture.clear();
            })
            .expect("Failed to color the texture.");
        Some(square_texture)
    } else {
        None
    }
}
mod score {
    /*
     * This module contains the code to handle high score reading and writing.
     * High score is stored as plaintext in the path defined in SAVE_FILE_PATH.
     * Number of high scores retained is defined in NB_HIGHSCORES
     */
    use std::fs::File;
    use std::io::{self, Read, Write};
    const SAVE_FILE_PATH: &str = "scores.txt";
    const NB_HIGHSCORES: usize = 5;

    fn write_into_file(content: String, file_name: &str) -> io::Result<()> {
        let mut f = File::create(file_name)?;
        f.write_all(content.as_bytes())
    }

    fn read_from_file(file_name: &str) -> io::Result<String> {
        let mut f = File::open(file_name)?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        Ok(content)
    }

    fn slice_to_string(slice: &[u32]) -> String {
        slice
            .iter()
            .map(|highscore| highscore.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub fn save_highscore_and_lines(highscores: &[u32], number_of_lines: &[u32]) -> bool {
        let s_highscore = slice_to_string(highscores);
        let s_number_of_lines = slice_to_string(number_of_lines);
        write_into_file(
            format!("{}\n{}\n", s_highscore, s_number_of_lines),
            SAVE_FILE_PATH,
        )
        .is_ok()
    }

    fn line_to_slice(line: &str) -> Vec<u32> {
        line.split(' ')
            .filter_map(|nb| nb.parse::<u32>().ok())
            .collect()
    }

    pub fn load_highscores_and_lines() -> Option<(Vec<u32>, Vec<u32>)> {
        if let Ok(content) = read_from_file(SAVE_FILE_PATH) {
            let mut lines = content
                .splitn(2, '\n')
                .map(line_to_slice)
                .collect::<Vec<_>>();
            if lines.len() == 2 {
                let (number_lines, highscore) = (lines.pop().unwrap(), lines.pop().unwrap());
                return Some((highscore, number_lines));
            }
        }
        None
    }

    pub fn update_vec(v: &mut Vec<u32>, value: u32) -> bool {
        if v.len() < NB_HIGHSCORES {
            v.push(value);
            v.sort();
            true
        } else {
            for entry in v.iter_mut() {
                if value > *entry {
                    *entry = value;
                    return true;
                }
            }
            false
        }
    }
}

type Piece = Vec<Vec<u8>>;
type States = Vec<Piece>;
#[derive(Clone)]
struct Tetrimino {
    states: States,
    x: isize,
    y: usize,
    current_state: u8,
}

impl Tetrimino {
    fn rotate(&mut self, game_map: &[Vec<u8>]) {
        let mut tmp_state = self.current_state + 1;
        if tmp_state >= self.states.len() as u8 {
            tmp_state = 0;
        }
        let x_pos = [0, -1, 1, -2, 2, -3];
        for x in x_pos.iter() {
            if self.test_position(game_map, tmp_state, self.x + x, self.y) {
                self.current_state = tmp_state;
                self.x += *x;
                break;
            }
        }
    }

    fn test_position(&self, game_map: &[Vec<u8>], tmp_state: u8, x: isize, y: usize) -> bool {
        for decal_y in 0..4 {
            for decal_x in 0..4 {
                let x = x + decal_x;
                if self.states[tmp_state as usize][decal_y][decal_x as usize] != 0
                    && (y + decal_y >= game_map.len()
                        || x < 0
                        || x as usize >= game_map[y + decal_y].len()
                        || game_map[y + decal_y][x as usize] != 0)
                {
                    return false;
                }
            }
        }
        true
    }

    fn test_current_position(&self, game_map: &[Vec<u8>]) -> bool {
        self.test_position(game_map, self.current_state, self.x, self.y)
    }

    fn change_position(&mut self, game_map: &[Vec<u8>], new_x: isize, new_y: usize) -> bool {
        if self.test_position(game_map, self.current_state, new_x, new_y) {
            self.x = new_x;
            self.y = new_y;
            return true;
        }
        false
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TetriminoTypes {
    TetriminoI,
    TetriminoJ,
    TetriminoL,
    TetriminoO,
    TetriminoS,
    TetriminoT,
    TetriminoZ,
}

impl Distribution<TetriminoTypes> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TetriminoTypes {
        match rng.gen_range(0..=6) {
            0 => TetriminoTypes::TetriminoI,
            1 => TetriminoTypes::TetriminoJ,
            2 => TetriminoTypes::TetriminoL,
            3 => TetriminoTypes::TetriminoO,
            4 => TetriminoTypes::TetriminoS,
            5 => TetriminoTypes::TetriminoT,
            _ => TetriminoTypes::TetriminoZ,
        }
    }
}

impl TetriminoTypes {
    fn generate(self) -> Tetrimino {
        match self {
            Self::TetriminoI => Tetrimino {
                states: vec![
                    vec![
                        vec![1, 1, 1, 1],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 1, 0, 0],
                        vec![0, 1, 0, 0],
                        vec![0, 1, 0, 0],
                        vec![0, 1, 0, 0],
                    ],
                ],
                x: 4,
                y: 0,
                current_state: 0,
            },
            Self::TetriminoJ => Tetrimino {
                states: vec![
                    vec![
                        vec![2, 2, 2, 0],
                        vec![0, 0, 2, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![2, 2, 0, 0],
                        vec![2, 0, 0, 0],
                        vec![2, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![2, 0, 0, 0],
                        vec![2, 2, 2, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 2, 0, 0],
                        vec![0, 2, 0, 0],
                        vec![2, 2, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                ],
                x: 4,
                y: 0,
                current_state: 0,
            },
            Self::TetriminoL => Tetrimino {
                states: vec![
                    vec![
                        vec![3, 3, 3, 0],
                        vec![3, 0, 0, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![3, 3, 0, 0],
                        vec![0, 3, 0, 0],
                        vec![0, 3, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 0, 3, 0],
                        vec![3, 3, 3, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![3, 0, 0, 0],
                        vec![3, 0, 0, 0],
                        vec![3, 3, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                ],
                x: 4,
                y: 0,
                current_state: 0,
            },
            Self::TetriminoO => Tetrimino {
                states: vec![vec![
                    vec![4, 4, 0, 0],
                    vec![4, 4, 0, 0],
                    vec![0, 0, 0, 0],
                    vec![0, 0, 0, 0],
                ]],
                x: 5,
                y: 0,
                current_state: 0,
            },
            Self::TetriminoS => Tetrimino {
                states: vec![
                    vec![
                        vec![0, 5, 5, 0],
                        vec![5, 5, 0, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 5, 0, 0],
                        vec![0, 5, 5, 0],
                        vec![0, 0, 5, 0],
                        vec![0, 0, 0, 0],
                    ],
                ],
                x: 4,
                y: 0,
                current_state: 0,
            },
            Self::TetriminoT => Tetrimino {
                states: vec![
                    vec![
                        vec![6, 6, 6, 0],
                        vec![0, 6, 0, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 6, 0, 0],
                        vec![6, 6, 0, 0],
                        vec![0, 6, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 6, 0, 0],
                        vec![6, 6, 6, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 6, 0, 0],
                        vec![0, 6, 6, 0],
                        vec![0, 6, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                ],
                x: 4,
                y: 0,
                current_state: 0,
            },
            Self::TetriminoZ => Tetrimino {
                states: vec![
                    vec![
                        vec![7, 7, 0, 0],
                        vec![0, 7, 7, 0],
                        vec![0, 0, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                    vec![
                        vec![0, 0, 7, 0],
                        vec![0, 7, 7, 0],
                        vec![0, 7, 0, 0],
                        vec![0, 0, 0, 0],
                    ],
                ],
                x: 4,
                y: 0,
                current_state: 0,
            },
        }
    }
}

struct Tetris {
    game_map: Vec<Vec<u8>>,
    current_level: u32,
    score: u32,
    nb_lines: u32,
    current_piece: Option<Tetrimino>,
    last_piece: Option<TetriminoTypes>,
}

impl Tetris {
    fn new() -> Tetris {
        let mut game_map: Vec<Vec<u8>> = Vec::new();
        for _ in 0..16 {
            game_map.push(Vec::from([0; 10]));
        }
        Tetris {
            game_map,
            current_level: 1,
            score: 0,
            nb_lines: 0,
            current_piece: None,
            last_piece: None,
        }
    }

    fn create_next_tetrimino(&mut self) {
        let mut next: TetriminoTypes = rand::random();
        if let Some(last) = self.last_piece {
            while next == last {
                next = rand::random();
            }
        }
        self.last_piece = Some(next);
        self.current_piece = Some(TetriminoTypes::generate(next));
    }

    fn update_score(&mut self, to_add: u32) {
        self.score += to_add;
    }

    fn check_lines(&mut self) {
        let mut y = 0;
        let mut score_add = 0;

        while y < self.game_map.len() {
            let mut complete = true;

            for x in &self.game_map[y] {
                if x == &0 {
                    complete = false;
                    break;
                }
            }
            if complete {
                score_add += self.current_level;
                self.game_map.remove(y);
            } else {
                y += 1;
            }
        }
        if self.game_map.is_empty() {
            // A "tetris"
            score_add += 1000;
        }
        self.update_score(score_add);

        while self.game_map.len() < 16 {
            self.increase_line();
            self.game_map.insert(0, Vec::from([0; 10]));
        }
    }

    fn make_permanent(&mut self) {
        let mut to_add = 0;
        if let Some(ref mut piece) = self.current_piece {
            let mut shift_y = 0;

            while shift_y < piece.states[piece.current_state as usize].len()
                && piece.y + shift_y < self.game_map.len()
            {
                let mut shift_x = 0;

                while shift_x < piece.states[piece.current_state as usize][shift_y].len()
                    && (piece.x + shift_x as isize)
                        < self.game_map[piece.y + shift_y].len() as isize
                {
                    if piece.states[piece.current_state as usize][shift_y][shift_x] != 0 {
                        let x = piece.x + shift_x as isize;
                        self.game_map[piece.y + shift_y][x as usize] =
                            piece.states[piece.current_state as usize][shift_y][shift_x];
                    }
                    shift_x += 1;
                }
                shift_y += 1;
            }
            to_add += self.current_level;
        }
        self.update_score(to_add);
        self.check_lines();
        self.current_piece = None;
    }

    fn increase_line(&mut self) {
        self.nb_lines += 1;
        if self.nb_lines > LEVEL_LINES[self.current_level as usize - 1] {
            self.current_level += 1;
        }
    }
}

fn is_time_over(tetris: &Tetris, timer: &SystemTime) -> bool {
    match timer.elapsed() {
        Ok(elapsed) => {
            let millis = elapsed.as_secs() as u32 * 1000 + elapsed.subsec_millis();
            millis > LEVEL_TIMES[tetris.current_level as usize - 1]
        }
        Err(_) => false,
    }
}

fn handle_events(
    tetris: &mut Tetris,
    quit: &mut bool,
    timer: &mut SystemTime,
    event_pump: &mut sdl2::EventPump,
) -> bool {
    let mut make_permanant = false;
    if let Some(ref mut piece) = tetris.current_piece {
        let mut tmp_x = piece.x;
        let mut tmp_y = piece.y;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    *quit = true;
                    break;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    *timer = SystemTime::now();
                    tmp_y += 1;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    tmp_x += 1;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    tmp_x -= 1;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    piece.rotate(&tetris.game_map);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    let x = piece.x;
                    let mut y = piece.y;
                    while piece.change_position(&tetris.game_map, x, y + 1) {
                        y += 1;
                    }
                }
                _ => {}
            }
        }
        if !make_permanant
            && !piece.change_position(&tetris.game_map, tmp_x, tmp_y)
            && tmp_y != piece.y
        {
            make_permanant = true;
        }
    }
    if make_permanant {
        tetris.make_permanent();
        *timer = SystemTime::now();
    }
    make_permanant
}

fn print_game_info(tetris: &Tetris) {
    let mut new_highest_highscore = true;
    let mut new_highest_lines_sent = true;
    if let Some((mut highscores, mut lines_sent)) = score::load_highscores_and_lines() {
        new_highest_highscore = score::update_vec(&mut highscores, tetris.score);
        new_highest_lines_sent = score::update_vec(&mut lines_sent, tetris.nb_lines);
        if new_highest_highscore || new_highest_lines_sent {
            score::save_highscore_and_lines(&highscores, &lines_sent);
        }
    } else {
        score::save_highscore_and_lines(&[tetris.score], &[tetris.nb_lines]);
    }

    println!("Game over...");
    println!(
        "Score:            {}{}",
        tetris.score,
        if new_highest_highscore {
            " [NEW HIGHSCORE]"
        } else {
            ""
        }
    );
    println!(
        "Number of lines:  {}{}",
        tetris.nb_lines,
        if new_highest_lines_sent {
            " [NEW HIGHSCORE]"
        } else {
            ""
        }
    );
    println!("Current level:    {}", tetris.current_level);
}

fn main() {
    let sdl_context = sdl2::init().expect("SDL initalizaton failed.");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to find SDL video subsystem.");
    let width = 600;
    let height = 800;

    let mut tetris = Tetris::new();
    let mut timer = SystemTime::now();
    // main event loop
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Failed to get SDL event pump.");
    let grid_x = (width - TETRIS_HEIGHT as u32 * 10) as i32 / 2;
    let grid_y = (height - TETRIS_HEIGHT as u32 * 16) as i32 / 2;

    let window = video_subsystem
        .window("Tetris", width, height)
        .position_centered()
        .opengl()
        .build()
        .expect("Failed to create window.");

    let mut canvas = window
        .into_canvas()
        .target_texture()
        .present_vsync()
        .build()
        .expect("Failed to convert window into canvas.");

    let texture_creator: TextureCreator<_> = canvas.texture_creator();

    let grid = create_texture_rect(
        &mut canvas,
        &texture_creator,
        0,
        0,
        0,
        TETRIS_HEIGHT as u32 * 10,
        TETRIS_HEIGHT as u32 * 16,
    )
    .expect("Failed to create grid texture.");

    let border = create_texture_rect(
        &mut canvas,
        &texture_creator,
        255,
        255,
        255,
        TETRIS_HEIGHT as u32 * 10 + 20,
        TETRIS_HEIGHT as u32 * 16 + 20,
    )
    .expect("Failed to create border texture.");

    macro_rules! texture {
        ($r:expr, $g:expr, $b:expr) => {
            create_texture_rect(
                &mut canvas,
                &texture_creator,
                $r,
                $g,
                $b,
                TETRIS_HEIGHT as u32,
                TETRIS_HEIGHT as u32,
            )
            .unwrap()
        };
    }

    let textures = vec![
        texture!(255, 69, 69),
        texture!(255, 220, 69),
        texture!(237, 150, 37),
        texture!(171, 99, 237),
        texture!(77, 149, 239),
        texture!(39, 218, 225),
        texture!(45, 216, 47),
    ];

    loop {
        if is_time_over(&tetris, &timer) {
            let mut make_permanent = false;
            if let Some(ref mut piece) = tetris.current_piece {
                let x = piece.x;
                let y = piece.y + 1;
                make_permanent = !piece.change_position(&tetris.game_map, x, y);
            }
            if make_permanent {
                tetris.make_permanent();
            }
            timer = SystemTime::now();
        }

        canvas.set_draw_color(Color::RGB(255, 0, 0));
        canvas.clear();

        canvas
            .copy(
                &border,
                None,
                Rect::new(
                    (width - TETRIS_HEIGHT as u32 * 10) as i32 / 2 - 10,
                    (height - TETRIS_HEIGHT as u32 * 16) as i32 / 2 - 10,
                    TETRIS_HEIGHT as u32 * 10 + 20,
                    TETRIS_HEIGHT as u32 * 16 + 20,
                ),
            )
            .expect("Couldn't copy border texture into window.");

        canvas
            .copy(
                &grid,
                None,
                Rect::new(
                    (width - TETRIS_HEIGHT as u32 * 10) as i32 / 2,
                    (height - TETRIS_HEIGHT as u32 * 16) as i32 / 2,
                    TETRIS_HEIGHT as u32 * 10,
                    TETRIS_HEIGHT as u32 * 16,
                ),
            )
            .expect("Couldn't copy grid texture into window.");

        if tetris.current_piece.is_none() {
            tetris.create_next_tetrimino();
            if !tetris
                .current_piece
                .as_ref()
                .unwrap()
                .test_current_position(&tetris.game_map)
            {
                print_game_info(&tetris);
                break;
            }
        }

        let mut quit = false;
        if !handle_events(&mut tetris, &mut quit, &mut timer, &mut event_pump) {
            if let Some(ref mut piece) = tetris.current_piece {
                for (line_nb, line) in piece.states[piece.current_state as usize]
                    .iter()
                    .enumerate()
                {
                    for (case_nb, case) in line.iter().enumerate() {
                        if *case == 0 {
                            continue;
                        }

                        canvas
                            .copy(
                                &textures[*case as usize - 1],
                                None,
                                Rect::new(
                                    grid_x
                                        + (piece.x + case_nb as isize) as i32
                                            * TETRIS_HEIGHT as i32,
                                    grid_y + (piece.y + line_nb) as i32 * TETRIS_HEIGHT as i32,
                                    TETRIS_HEIGHT as u32,
                                    TETRIS_HEIGHT as u32,
                                ),
                            )
                            .expect("Failed to copy tetrimino texture to window.")
                    }
                }
            }
        }

        if quit {
            print_game_info(&tetris);
            break;
        }
        for (line_nb, line) in tetris.game_map.iter().enumerate() {
            for (case_nb, case) in line.iter().enumerate() {
                if *case == 0 {
                    continue;
                }
                canvas
                    .copy(
                        &textures[*case as usize - 1],
                        None,
                        Rect::new(
                            grid_x + case_nb as i32 * TETRIS_HEIGHT as i32,
                            grid_y + line_nb as i32 * TETRIS_HEIGHT as i32,
                            TETRIS_HEIGHT as u32,
                            TETRIS_HEIGHT as u32,
                        ),
                    )
                    .expect("Failed to copy tetrimino texture to window.");
            }
        }
        canvas.present();

        sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
