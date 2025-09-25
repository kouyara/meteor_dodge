use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent};

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn main_with_hook() {
    console_error_panic_hook::set_once();
    start();
}

#[cfg(not(feature = "console_error_panic_hook"))]
#[wasm_bindgen(start)]
pub fn main_wo_hook() {
    start();
}

#[derive(Clone, Copy)]
struct Rect { x: f64, y: f64, w: f64, h: f64 }
impl Rect {
    fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && self.x + self.w > o.x && self.y < o.y + o.h && self.y + self.h > o.y
    }
}

struct Meteor { r: Rect, vy: f64 }

struct Input { left: bool, right: bool }

struct Game {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
    player: Rect,
    meteors: Vec<Meteor>,
    spawn_timer: f64,
    score: f64,
    speed: f64,
    input: Input,
    over: bool,
    last_t: f64,
}

impl Game {
    fn new(ctx: CanvasRenderingContext2d, canvas: &HtmlCanvasElement, display_width: f64, display_height: f64) -> Self {
        let width = display_width;
        let height = display_height;
        Self {
            ctx,
            width,
            height,
            player: Rect { x: width * 0.5 - 15.0, y: height - 40.0, w: 30.0, h: 20.0 },
            meteors: Vec::new(),
            spawn_timer: 0.0,
            score: 0.0,
            speed: 120.0,
            input: Input { left: false, right: false },
            over: false,
            last_t: now_ms(),
        }
    }

    fn reset(&mut self) {
        self.player.x = self.width * 0.5 - 15.0;
        self.meteors.clear();
        self.spawn_timer = 0.0;
        self.score = 0.0;
        self.speed = 120.0;
        self.over = false;
    }

    fn update(&mut self, dt: f64) {
        if self.over { return; }

        // 入力
        let move_speed = 220.0;
        if self.input.left { self.player.x -= move_speed * dt; }
        if self.input.right { self.player.x += move_speed * dt; }
        self.player.x = self.player.x.clamp(0.0, self.width - self.player.w);

        // スポーン
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            self.spawn_timer = (0.8_f64.max(1.2 - self.score * 0.001)).max(0.15);
            let x = rand_between(0.0, self.width - 14.0);
            let size = rand_between(10.0, 24.0);
            let vy = rand_between(self.speed, self.speed + 160.0);
            self.meteors.push(Meteor { r: Rect { x, y: -size, w: size, h: size }, vy });
        }

        // 落下 & 当たり判定
        for m in &mut self.meteors { m.r.y += m.vy * dt; }
        if self.meteors.iter().any(|m| m.r.intersects(&self.player)) {
            self.over = true;
        }
        // 画面外を掃除
        self.meteors.retain(|m| m.r.y < self.height + 60.0);

        // スコア & 難易度
        self.score += dt * 100.0;
        self.speed = 120.0 + (self.score * 0.6);
    }

    fn draw(&self) {
        let c = &self.ctx;
        c.set_fill_style(&"#0b1020".into());
        c.fill_rect(0.0, 0.0, self.width, self.height);

        // 星っぽい背景：軽いちらつき
        c.set_fill_style(&"#111a33".into());
        for i in 0..30 { let x = (i * 53 % 997) as f64; c.fill_rect((x*7.0)%self.width, (x*13.0)%self.height, 1.0, 1.0); }

        // プレイヤー（明るい緑色で目立つように）
        c.set_fill_style(&"#00ff88".into());
        c.fill_rect(self.player.x, self.player.y, self.player.w, self.player.h);
        
        // プレイヤーの輪郭を追加（より見やすくするため）
        c.set_stroke_style(&"#ffffff".into());
        c.set_line_width(1.0);
        c.stroke_rect(self.player.x, self.player.y, self.player.w, self.player.h);

        // 隕石
        c.set_fill_style(&"#e85d75".into());
        for m in &self.meteors { c.fill_rect(m.r.x, m.r.y, m.r.w, m.r.h); }

        // スコア
        c.set_fill_style(&"#cce1ff".into());
        c.set_font("16px ui-monospace, Menlo, Consolas, monospace");
        let _ = c.fill_text(&format!("SCORE: {:04}", self.score as i32), 10.0, 22.0);
        
        // デバッグ情報（プレイヤー位置と画面サイズ）
        let _ = c.fill_text(&format!("Player: ({:.0}, {:.0})", self.player.x, self.player.y), 10.0, 42.0);
        let _ = c.fill_text(&format!("Screen: {:.0}x{:.0}", self.width, self.height), 10.0, 62.0);

        if self.over {
            c.set_fill_style(&"rgba(0,0,0,0.5)".into());
            c.fill_rect(0.0, 0.0, self.width, self.height);
            c.set_fill_style(&"#ffffff".into());
            c.set_font("bold 28px ui-sans-serif, system-ui");
            let _ = c.fill_text("GAME OVER", self.width*0.5 - 90.0, self.height*0.5 - 8.0);
            c.set_font("16px ui-monospace, Menlo, Consolas, monospace");
            let _ = c.fill_text("Press R to retry", self.width*0.5 - 85.0, self.height*0.5 + 20.0);
        }
    }
}

fn now_ms() -> f64 {
    window().unwrap()
        .performance().unwrap()
        .now()
}

// シンプルなXorshift（JSのMath.randomを呼ばずにRust側で）
static mut SEED: u64 = 0x1234_5678_90ab_cdef;
fn rand_f64() -> f64 { unsafe {
    SEED ^= SEED << 7; SEED ^= SEED >> 9; SEED ^= SEED << 8;
    ((SEED & 0xFFFF_FFFF) as f64) / (u32::MAX as f64)
}}
fn rand_between(a: f64, b: f64) -> f64 { a + (b - a) * rand_f64() }

fn canvas_and_ctx() -> (HtmlCanvasElement, CanvasRenderingContext2d) {
    let win = window().unwrap();
    let doc = win.document().unwrap();
    let canvas = doc
        .get_element_by_id("game").unwrap()
        .dyn_into::<HtmlCanvasElement>().unwrap();
    let ctx = canvas
        .get_context("2d").unwrap().unwrap()
        .dyn_into::<CanvasRenderingContext2d>().unwrap();
    (canvas, ctx)
}

fn add_key_listeners(game_rc: std::rc::Rc<GameCell>) {
    let win = window().unwrap();
    let handler_down = {
        let g = game_rc.clone();
        Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            if ["ArrowLeft", "ArrowRight", "Space"].contains(&e.key().as_str()) { e.prevent_default(); }
            let mut inner = g.0.borrow_mut();
            match e.key().as_str() {
                "ArrowLeft" | "a" | "A" => inner.input.left = true,
                "ArrowRight" | "d" | "D" => inner.input.right = true,
                "r" | "R" => if inner.over { inner.reset(); },
                _ => {}
            }
        })
    };
    let handler_up = {
        let g = game_rc.clone();
        Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            let mut inner = g.0.borrow_mut();
            match e.key().as_str() {
                "ArrowLeft" | "a" | "A" => inner.input.left = false,
                "ArrowRight" | "d" | "D" => inner.input.right = false,
                _ => {}
            }
        })
    };

    win.add_event_listener_with_callback("keydown", handler_down.as_ref().unchecked_ref()).unwrap();
    win.add_event_listener_with_callback("keyup", handler_up.as_ref().unchecked_ref()).unwrap();
    handler_down.forget();
    handler_up.forget();
}

// RefCell を JS 側に乗せるためのラッパ
#[wasm_bindgen]
pub struct GameCell(std::cell::RefCell<Game>);

#[wasm_bindgen]
impl GameCell {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameCell {
        let (canvas, ctx) = canvas_and_ctx();
        // デバイスピクセル比に応じてリサイズ（高DPIディスプレイでクッキリ）
        let dpr = window().unwrap().device_pixel_ratio();
        let client_width = canvas.client_width() as f64;
        let client_height = canvas.client_height() as f64;
        let w = (client_width * dpr).round() as u32;
        let h = (client_height * dpr).round() as u32;
        canvas.set_width(w);
        canvas.set_height(h);
        ctx.scale(dpr, dpr).ok();

        let mut g = Game::new(ctx, &canvas, client_width, client_height);
        g.last_t = now_ms();
        GameCell(std::cell::RefCell::new(g))
    }

    pub fn tick(&self) {
        let mut g = self.0.borrow_mut();
        let t = now_ms();
        let dt = ((t - g.last_t) / 1000.0).min(0.033); // 30msまでにクランプ
        g.last_t = t;
        g.update(dt);
        g.draw();
    }
}

fn start() {
    let game = std::rc::Rc::new(GameCell::new());
    add_key_listeners(game.clone());

    // requestAnimationFrame ループ
    let f: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut()>>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let g_clone = game.clone();
    let cb = {
        let f = f.clone();
        Closure::wrap(Box::new(move || {
            g_clone.tick();
            let window = window().unwrap();
            window.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
        }) as Box<dyn FnMut()>)
    };

    *f.borrow_mut() = Some(cb);
    let window = window().unwrap();
    window.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
}