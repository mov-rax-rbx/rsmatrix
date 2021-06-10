#![forbid(unsafe_code)]

use crossterm::{cursor, event, style, terminal, ExecutableCommand, QueueableCommand};

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::{fs, io, time};

use notify::{
    event::ModifyKind, Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

use rand::prelude::*;

mod config_parser;
mod rmatrix;

use config_parser::*;
use rmatrix::*;

#[derive(Debug)]
struct RmatrixCrosstermRender<'rm, R>
where
    R: Rng,
{
    rmatrix: &'rm mut Rmatrix<R>,
}

impl<'rm, R> RmatrixCrosstermRender<'rm, R>
where
    R: Rng,
{
    fn render<O>(&mut self, out: &mut O) -> crossterm::Result<()>
    where
        O: Write + QueueableCommand,
    {
        let need_double_buffer = !self.rmatrix.is_default_rain
            || self.rmatrix.interpolate_color_koef.is_some()
            || self.rmatrix.start_gradient_color.is_some();
        let mut double_buffer = if need_double_buffer {
            Some(vec![
                0u8;
                self.rmatrix.width as usize
                    * (self.rmatrix.height as usize + 1)
            ])
        } else {
            None
        };

        if self.rmatrix.is_bold {
            out.queue(style::SetAttribute(style::Attribute::Bold))?;
        }

        let start_color = self
            .rmatrix
            .start_gradient_color
            .clone()
            .unwrap_or_default();
        let color = self.rmatrix.color.sub(&start_color);

        for head in self.rmatrix.rains.iter() {
            let start_y = head.y.saturating_sub(head.length);

            let need_y = start_y.saturating_sub(head.speed);
            for y in need_y..start_y {
                out.queue(cursor::MoveTo(head.x, y))?
                    .queue(style::Print(' '))?;
            }

            let (start_color, color) = if let Some(brightnes) = head.brightnes {
                (
                    start_color.interpolate(brightnes),
                    color.interpolate(brightnes),
                )
            } else {
                (start_color.clone(), color.clone())
            };

            if self.rmatrix.interpolate_color_koef.is_none()
                && self.rmatrix.start_gradient_color.is_none()
            {
                out.queue(style::SetForegroundColor(color.tuple().into()))?;
            }

            if let Some(double_buffer) = double_buffer.as_mut() {
                let mut double_buffer_idx =
                    start_y as usize * self.rmatrix.width as usize + head.x as usize;

                let interpolate_koef = self.rmatrix.interpolate_color_koef.unwrap_or(1.0);

                let ddc = 1.0 / head.length as f32 * interpolate_koef;
                let not_visible_len = head.length - (head.y - start_y);
                let mut walked_len = ddc * not_visible_len as f32;
                for y in start_y..head.y {
                    if y > self.rmatrix.height {
                        break;
                    }

                    let symbl = if self.rmatrix.is_default_rain {
                        let pos =
                            (head.symbl_pos as usize + y as usize) % self.rmatrix.symbls.len();
                        self.rmatrix.symbls[pos]
                    } else {
                        *self
                            .rmatrix
                            .symbls
                            .choose(&mut self.rmatrix.rng)
                            .expect("Invalid choose")
                    };

                    if self.rmatrix.interpolate_color_koef.is_some()
                        || self.rmatrix.start_gradient_color.is_some()
                    {
                        let dc = walked_len;
                        walked_len += ddc;

                        let color = start_color.add(&color.interpolate(dc));
                        out.queue(style::SetForegroundColor(color.tuple().into()))?;
                    }

                    if double_buffer[double_buffer_idx] == 0 {
                        double_buffer[double_buffer_idx] = 1;
                        out.queue(cursor::MoveTo(head.x, y))?
                            .queue(style::Print(symbl))?;
                    }
                    double_buffer_idx += self.rmatrix.width as usize;
                }
            } else {
                let need_y = head.y.saturating_sub(head.speed);
                let last_y = head.y.min(self.rmatrix.height);
                for y in need_y..=last_y {
                    let pos = (head.symbl_pos as usize + y as usize) % self.rmatrix.symbls.len();
                    let symbl = self.rmatrix.symbls[pos];

                    out.queue(cursor::MoveTo(head.x, y))?
                        .queue(style::Print(symbl))?;
                }
            }

            if head.y > self.rmatrix.height {
                continue;
            }

            let double_buffer_idx = head.y as usize * self.rmatrix.width as usize + head.x as usize;
            if let Some(double_buffer) = double_buffer.as_mut() {
                if double_buffer[double_buffer_idx] == 0 {
                    double_buffer[double_buffer_idx] = 1;
                } else {
                    continue;
                }
            }

            let head_symbl = if self.rmatrix.is_default_rain {
                let pos = (head.symbl_pos as usize + head.y as usize) % self.rmatrix.symbls.len();
                self.rmatrix.symbls[pos]
            } else {
                self.rmatrix.symbls[head.symbl_pos as usize]
            };

            if let Some(head_color) = self.rmatrix.head_color.clone() {
                out.queue(style::SetForegroundColor(head_color.tuple().into()))?;
            }
            out.queue(cursor::MoveTo(head.x, head.y))?
                .queue(style::Print(head_symbl))?;
        }

        out.flush()?;
        Ok(())
    }
}

fn try_set_config_param<R: Rng>(
    rmatrix: &mut Rmatrix<R>,
    param: ConfigParam,
) -> Result<(), String> {
    let (name, value) = param.split();
    match name.to_lowercase().as_str() {
        "speed" => {
            match value {
                ConfigVal::Range(box_v1, box_v2) => {
                    if let (ConfigVal::Num(v1), ConfigVal::Num(v2)) = (*box_v1, *box_v2) {
                        if v1 < v2 {
                            rmatrix.speed = v1 as u16..v2 as u16;
                            return Ok(());
                        }
                    }
                }
                ConfigVal::Nil => return Ok(()),
                _ => {}
            }

            Err(format!("Speed is range of number (`1..3`)."))
        }
        "length" => {
            match value {
                ConfigVal::Range(box_v1, box_v2) => {
                    if let (ConfigVal::Num(v1), ConfigVal::Num(v2)) = (*box_v1, *box_v2) {
                        if v1 < v2 {
                            rmatrix.len = v1 as u16..v2 as u16;
                            return Ok(());
                        }
                    }
                }
                ConfigVal::Nil => return Ok(()),
                _ => {}
            }

            Err(format!("Length is range of number (`1..3`)."))
        }
        "color" => {
            match value {
                ConfigVal::Range(box_v1, box_v2) => match (*box_v1, *box_v2) {
                    (ConfigVal::Tuple(start_color), ConfigVal::Tuple(end_color)) => {
                        if let [ConfigVal::Num(s1), ConfigVal::Num(s2), ConfigVal::Num(s3)] =
                            start_color[..]
                        {
                            if let [ConfigVal::Num(e1), ConfigVal::Num(e2), ConfigVal::Num(e3)] =
                                end_color[..]
                            {
                                rmatrix.start_gradient_color =
                                    Some(RColor::new(s1 as u8, s2 as u8, s3 as u8));
                                rmatrix.color = RColor::new(e1 as u8, e2 as u8, e3 as u8);
                                return Ok(());
                            }
                        }
                    }
                    _ => {}
                },
                ConfigVal::Tuple(box_v) => {
                    if let [ConfigVal::Num(c1), ConfigVal::Num(c2), ConfigVal::Num(c3)] = box_v[..]
                    {
                        rmatrix.start_gradient_color = None;
                        rmatrix.color = RColor::new(c1 as u8, c2 as u8, c3 as u8);
                        return Ok(());
                    }
                }
                ConfigVal::Nil => return Ok(()),
                _ => {}
            }

            Err(format!(
                "Color is range of tuple (`(0, 0, 0)..(0, 255, 0)`), tuple of number (`(0, 255, 0)`) or `nil`."
            ))
        }
        "head_color" => {
            match value {
                ConfigVal::Tuple(box_v) => {
                    if let [ConfigVal::Num(c1), ConfigVal::Num(c2), ConfigVal::Num(c3)] = box_v[..]
                    {
                        rmatrix.head_color = Some(RColor::new(c1 as u8, c2 as u8, c3 as u8));
                        return Ok(());
                    }
                }
                ConfigVal::Nil => {
                    rmatrix.head_color = None;
                    return Ok(());
                }
                _ => {}
            }

            Err(format!(
                "Head color is tuple of number (`(255, 255, 255)`) or `nil`."
            ))
        }
        "interpolate_color_koef" => {
            match value {
                ConfigVal::Num(v) => {
                    rmatrix.interpolate_color_koef = Some(v);
                    return Ok(());
                }
                ConfigVal::Nil => {
                    rmatrix.interpolate_color_koef = None;
                    return Ok(());
                }
                _ => {}
            }

            Err(format!(
                "Interpolate color koef is number (`1.25`) or `nil`."
            ))
        }
        "min_brightnes" => {
            match value {
                ConfigVal::Num(v) => {
                    rmatrix.min_brightnes = Some(v);
                    return Ok(());
                }
                ConfigVal::Nil => {
                    rmatrix.min_brightnes = None;
                    return Ok(());
                }
                _ => {}
            }

            Err(format!(
                "Interpolate color koef is number (`0.1`) or `nil`."
            ))
        }
        "density" => {
            match value {
                ConfigVal::Num(v) => {
                    rmatrix.density = v;
                    return Ok(());
                }
                _ => {}
            }

            Err(format!("Density is number (`0.7`)."))
        }
        "is_bold" => {
            match value {
                ConfigVal::Bool(b) => {
                    rmatrix.is_bold = b;
                    return Ok(());
                }
                _ => {}
            }
            Err(format!("Bold is bool (`true`)."))
        }
        "is_default_rain" => {
            match value {
                ConfigVal::Bool(b) => {
                    rmatrix.is_default_rain = b;
                    return Ok(());
                }
                _ => {}
            }
            Err(format!("Default rain is bool (`true`)."))
        }
        "delay" => {
            match value {
                ConfigVal::Num(v) => {
                    rmatrix.delay = time::Duration::from_millis(v as u64);
                    return Ok(());
                }
                _ => {}
            }
            Err(format!("Delay is number (`16`)."))
        }
        "utf8" => {
            match value {
                ConfigVal::Bool(b) => {
                    if b {
                        rmatrix.set_utf8();
                    } else {
                        rmatrix.set_ascii();
                    }
                    return Ok(());
                }
                _ => {}
            }
            Err(format!("utf8 is bool (`true`)."))
        }
        name => Err(format!("Unexpected variable name `{}`.", name)),
    }
}

fn rmatrix_from_config<R: Rng, W: Write>(
    config: &str,
    rmatrix: &mut Rmatrix<R>,
    err_writer: &mut Option<W>,
) {
    fn write_ignore<W: Write>(err_writer: &mut Option<W>, err: String) {
        err_writer
            .as_mut()
            .map(|write| write.write_all(err.as_bytes()).unwrap());
    }

    let config = fs::read_to_string(config);
    if let Ok(ref config) = config {
        let mut parser = ConfigParser::new(config);
        while let Some(res) = parser.parse() {
            match res {
                Ok(param) => {
                    let res = try_set_config_param(rmatrix, param);
                    if let Err(err) = res {
                        write_ignore(err_writer, err);
                    }
                }
                Err(err) => write_ignore(err_writer, format!("{}", err)),
            }
        }
    }
}

const CONFIG_NAME: &'static str = "config.rm";
const ERROR_CONFIG_NAME: &'static str = "config_error.txt";

fn main() -> crossterm::Result<()> {
    let mut config_error = fs::File::create(ERROR_CONFIG_NAME).ok();
    let mut rmatrix = Rmatrix::default();
    rmatrix_from_config(CONFIG_NAME, &mut rmatrix, &mut config_error);

    let rmatrix = Arc::new(Mutex::new(rmatrix));
    let cloned_rmatrix = Arc::clone(&rmatrix);

    let mut watcher: RecommendedWatcher =
        Watcher::new_immediate(move |result: Result<Event, Error>| {
            let event = result.unwrap();
            if event.kind == EventKind::Modify(ModifyKind::Any) {
                let mut config_error = fs::File::create(ERROR_CONFIG_NAME).ok();
                let mut new_rmatrix = cloned_rmatrix.lock().unwrap();
                rmatrix_from_config(CONFIG_NAME, &mut new_rmatrix, &mut config_error);
            }
        })
        .unwrap();

    watcher
        .watch(CONFIG_NAME, RecursiveMode::NonRecursive)
        .unwrap();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(cursor::Hide)?
        .execute(cursor::SavePosition)?;

    let (width, height) = terminal::size()?;
    rmatrix.lock().unwrap().resize(width, height);

    loop {
        if event::poll(rmatrix.lock().unwrap().delay())? {
            match event::read()? {
                event::Event::Key(event) => match event {
                    event::KeyEvent {
                        code: event::KeyCode::Esc,
                        ..
                    }
                    | event::KeyEvent {
                        code: event::KeyCode::Char('c'),
                        modifiers: event::KeyModifiers::CONTROL,
                    } => {
                        break;
                    }
                    _ => {}
                },
                event::Event::Resize(width, height) => {
                    stdout.queue(terminal::Clear(terminal::ClearType::All))?;
                    rmatrix.lock().unwrap().resize(width, height)
                }
                _ => {}
            }
        } else {
            rmatrix.lock().unwrap().update();
            rmatrix
                .lock()
                .unwrap()
                .to_crossterm_render()
                .render(&mut stdout)?;
        }
    }

    stdout
        .execute(terminal::LeaveAlternateScreen)?
        .execute(cursor::Show)?
        .execute(cursor::RestorePosition)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
