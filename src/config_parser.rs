#![forbid(unsafe_code)]

use core::fmt;
use core::iter::Peekable;

#[derive(Debug, Clone)]
pub struct ConfigParser<'s, I>
where
    I: Iterator<Item = (usize, char)>,
{
    slurp_config: &'s str,
    // Need some better abstraction (next must update line and column state).
    iter: Peekable<I>,

    line: usize,
    line_to_idx: usize,
}

#[derive(Debug, Clone)]
pub struct ConfigErr<'s> {
    msg: String,

    slurp_config: &'s str,

    line: usize,
    line_to_idx: usize,
    idx: Option<usize>,
}

impl fmt::Display for ConfigErr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.msg)?;

        let current_idx = self.idx.unwrap_or(self.slurp_config.len());
        let end = self.slurp_config[current_idx..]
            .char_indices()
            .find(|&(_, ch)| ch == '\n')
            .map(|(idx, _)| current_idx + idx)
            .unwrap_or(self.slurp_config.len());

        let prefix = format!("{} | ", self.line);
        write!(f, "{}", prefix)?;

        writeln!(f, "{}", &self.slurp_config[self.line_to_idx..end])?;

        for _ in 0..prefix.len() {
            write!(f, " ")?;
        }

        let len = self.slurp_config[self.line_to_idx..=current_idx]
            .chars()
            .count();
        for _ in 0..len {
            write!(f, "{}", '^')?;
        }
        writeln!(f)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigParam {
    name: String,
    val: ConfigVal,
}

impl ConfigParam {
    pub fn split(self) -> (String, ConfigVal) {
        (self.name, self.val)
    }
}

#[derive(Debug, Clone)]
pub enum ConfigVal {
    Num(f32),
    Tuple(Vec<ConfigVal>),
    Range(Box<ConfigVal>, Box<ConfigVal>),
    Bool(bool),
    Nil,
}

impl<'s> ConfigParser<'s, core::str::CharIndices<'s>> {
    pub fn new(slurp_config: &'s str) -> Self {
        Self {
            slurp_config,
            iter: slurp_config.char_indices().peekable(),
            line: 1,
            line_to_idx: 0,
        }
    }

    fn make_err(&self, msg: String, idx: Option<usize>) -> ConfigErr<'s> {
        ConfigErr {
            msg: msg,
            slurp_config: self.slurp_config,
            line: self.line,
            line_to_idx: self.line_to_idx,
            idx: idx,
        }
    }

    fn to_new_line(&mut self) {
        while let Some((idx, ch)) = self.iter.next() {
            if ch == '\n' {
                self.line += 1;
                self.line_to_idx = idx + 1;
                break;
            }
        }
    }

    fn skip<F: Fn(char) -> bool>(&mut self, f: F) {
        while let Some(&(idx, ch)) = self.iter.peek() {
            if !f(ch) {
                break;
            }

            if ch == '\n' {
                self.line += 1;
                self.line_to_idx = idx + 1;
            }
            let _ = self.iter.next();
        }
    }

    fn skip_n(&mut self, n: usize) {
        for _ in 0..n {
            if let Some(&(idx, ch)) = self.iter.peek() {
                if ch == '\n' {
                    self.line += 1;
                    self.line_to_idx = idx + 1;
                }
                let _ = self.iter.next();
            } else {
                break;
            }
        }
    }

    fn need(&mut self, need: &str) -> Result<(), ConfigErr<'s>> {
        for expect in need.chars() {
            let found = self.iter.next();

            if let Some((idx, found)) = found {
                if found == '\n' {
                    self.line += 1;
                    self.line_to_idx = idx + 1;
                }

                if expect != found {
                    let err = format!(
                        "error on line: {}, column: {}. Expected `{}`.",
                        self.line,
                        idx - self.line_to_idx,
                        need
                    );
                    return Err(self.make_err(err, Some(idx)));
                }
            } else {
                let err = format!(
                    "error on line: {}, column: {}. Expected `{}`.",
                    self.line,
                    self.slurp_config.len() - self.line_to_idx,
                    need
                );
                return Err(self.make_err(err, None));
            }
        }
        Ok(())
    }

    fn maybe(&self, maybe: &str) -> bool {
        let mut save_iter = self.iter.clone();
        for expect in maybe.chars() {
            let found = save_iter.next();

            if let Some((_, found)) = found {
                if expect != found {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    fn parse_variable(&mut self) -> Result<String, ConfigErr<'s>> {
        if let Some(&(start, _)) = self.iter.peek() {
            while let Some(&(end, ch)) = self.iter.peek() {
                if ch.is_whitespace() || ch == ':' || ch == '.' || ch == ',' || ch == ';' {
                    if start == end {
                        break;
                    }
                    return Ok(self.slurp_config[start..end].to_string());
                }

                if ch == '\n' {
                    self.line += 1;
                    self.line_to_idx = end + 1;
                }
                let _ = self.iter.next();
            }
        }

        let idx = if let Some(&(idx, _)) = self.iter.peek() {
            Some(idx)
        } else {
            None
        };
        let err = format!(
            "error on line: {}, column: {}. Expected `variable name`.",
            self.line,
            idx.unwrap_or(self.slurp_config.len()) - self.line_to_idx
        );
        Err(self.make_err(err, idx))
    }

    fn parse_num(&mut self) -> ConfigVal {
        if let Some(&(start, _)) = self.iter.peek() {
            let mut dot_cnt = 0;
            while let Some(&(end, ch)) = self.iter.peek() {
                if (!ch.is_digit(10) && ch != '.') || (ch == '.' && dot_cnt > 0) || self.maybe("..")
                {
                    let num = &self.slurp_config[start..end];
                    return ConfigVal::Num(num.parse().unwrap_or_default());
                }

                if ch == '.' {
                    dot_cnt += 1;
                }

                if ch == '\n' {
                    self.line += 1;
                    self.line_to_idx = end + 1;
                }
                let _ = self.iter.next();
            }
        }
        ConfigVal::Num(Default::default())
    }

    fn parse_tuple(&mut self) -> Result<ConfigVal, ConfigErr<'s>> {
        let mut tuple = Vec::with_capacity(3);

        let _ = self.need("(")?;
        loop {
            self.skip(char::is_whitespace);

            let val = self.parse_value()?;
            tuple.push(val);

            self.skip(char::is_whitespace);
            if let Some(&(_, ')')) = self.iter.peek() {
                let _ = self.iter.next();
                break;
            } else if let Some(&(_, ',')) = self.iter.peek() {
                let _ = self.iter.next();
            } else {
                let idx = if let Some(&(idx, _)) = self.iter.peek() {
                    Some(idx)
                } else {
                    None
                };

                let err = format!(
                    "error on line: {}, column: {}. Expected `,` or `)`.",
                    self.line,
                    idx.unwrap_or(self.slurp_config.len()) - self.line_to_idx
                );
                return Err(self.make_err(err, idx));
            }
        }
        Ok(ConfigVal::Tuple(tuple))
    }

    fn parse_value(&mut self) -> Result<ConfigVal, ConfigErr<'s>> {
        if let Some(&(idx, ch)) = self.iter.peek() {
            let value = if ch.is_digit(10) || ch == '.' {
                let num = self.parse_num();

                self.skip(char::is_whitespace);
                if self.maybe("..") {
                    let _ = self.iter.next();
                    let _ = self.iter.next();

                    self.skip(char::is_whitespace);
                    let num_last = self.parse_num();

                    ConfigVal::Range(Box::new(num), Box::new(num_last))
                } else {
                    num
                }
            } else if ch == '(' {
                let tuple = self.parse_tuple()?;

                self.skip(char::is_whitespace);
                if self.maybe("..") {
                    let _ = self.iter.next();
                    let _ = self.iter.next();

                    self.skip(char::is_whitespace);
                    let tuple_last = self.parse_tuple()?;

                    ConfigVal::Range(Box::new(tuple), Box::new(tuple_last))
                } else {
                    tuple
                }
            } else if self.maybe("true") {
                self.skip_n("true".len());
                ConfigVal::Bool(true)
            } else if self.maybe("false") {
                self.skip_n("false".len());
                ConfigVal::Bool(false)
            } else if (ch == 'N' && self.maybe("Nil")) || (ch == 'n' && self.maybe("nil")) {
                self.skip_n("nil".len());
                ConfigVal::Nil
            } else {
                let err = format!(
                    "error on line: {}, column: {}. Expected value.",
                    self.line,
                    idx - self.line_to_idx
                );
                return Err(self.make_err(err, Some(idx)));
            };

            Ok(value)
        } else {
            let idx = if let Some(&(idx, _)) = self.iter.peek() {
                Some(idx)
            } else {
                None
            };

            let err = format!(
                "error on line: {}. Expected value but found end of line.",
                self.line
            );
            return Err(self.make_err(err, idx));
        }
    }
    pub fn parse(&mut self) -> Option<Result<ConfigParam, ConfigErr>> {
        loop {
            self.skip(char::is_whitespace);
            match self.iter.peek() {
                Some((_, '#')) => self.to_new_line(),
                Some(_) => {
                    let variable = self.parse_variable();

                    let variable_name = match variable {
                        Ok(name) => name,
                        Err(variable_parse_err) => {
                            self.to_new_line();
                            return Some(Err(variable_parse_err));
                        }
                    };

                    self.skip(char::is_whitespace);
                    let is_assign = self.need("::");

                    match is_assign {
                        Err(assign_parse_err) => {
                            self.to_new_line();
                            return Some(Err(assign_parse_err));
                        }
                        _ => {}
                    }

                    self.skip(char::is_whitespace);
                    return Some(match self.parse_value() {
                        Ok(config_parse_val) => Ok(ConfigParam {
                            name: variable_name,
                            val: config_parse_val,
                        }),
                        Err(config_parse_err) => {
                            self.to_new_line();
                            Err(config_parse_err)
                        }
                    });
                }
                None => return None,
            }
        }
    }
}
