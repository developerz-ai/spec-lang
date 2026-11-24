//! One game being played: the machine from `start` on. A refused tick ends the run; any other halt stops the app.

use super::script::Hold;
use super::shape::Shape;
use crate::machine::Machine;
use crate::rt::data::Data;
use crate::rt::{Halt, R, stop};

#[derive(Debug)]
pub struct Session<'s> {
    shape: &'s Shape,
    machine: Machine,
    ticks: u64,
    over: bool,
    /// How much of `out.text` was taken already.
    taken: usize,
}

impl<'s> Session<'s> {
    pub fn start(shape: &'s Shape, level: Option<Data>) -> R<Self> {
        let mut machine = Machine::new()?;
        let args = if shape.level().is_some() { vec![level] } else { Vec::new() };
        (shape.start.call)(&mut machine, args)?;
        Ok(Session { shape, machine, ticks: 0, over: false, taken: 0 })
    }

    pub fn over(&self) -> bool {
        self.over
    }

    pub fn tick(&mut self, pad: &Data) -> R<()> {
        match (self.shape.tick.call)(&mut self.machine, vec![Some(pad.clone())]) {
            Ok(_) => self.ticks += 1,
            Err(Halt::Fail(_)) => self.over = true,
            Err(h) => return Err(h),
        }
        Ok(())
    }

    /// Every tick of the script while the game goes on, up to tick `upto` (`None`: the end of the script).
    pub fn play(&mut self, holds: &[Hold], upto: Option<u64>) -> R<()> {
        for hold in holds {
            for _ in 0..hold.ticks {
                if self.over || upto.is_some_and(|t| self.ticks >= t) {
                    return Ok(());
                }
                self.tick(&hold.pad)?;
            }
        }
        Ok(())
    }

    /// The lines sent with `out.line` since the last take, each ending in a newline.
    pub fn lines(&mut self) -> R<String> {
        let Some(read) = self.shape.app.read("out", "text") else { return Ok(String::new()) };
        let Data::Text(all) = (read.read)(&mut self.machine)? else { return Ok(String::new()) };
        let new = all.get(self.taken..).unwrap_or_default().to_string();
        self.taken = all.len();
        Ok(new)
    }

    pub fn input(&mut self) -> R<Data> {
        (self.shape.input.call)(&mut self.machine, Vec::new())
    }

    pub fn frame(&mut self) -> R<()> {
        (self.shape.frame.call)(&mut self.machine, Vec::new()).map(|_| ())
    }

    /// The frame as text, one line per item.
    pub fn dump(&mut self) -> R<String> {
        let Data::List(lines) = (self.shape.dump.call)(&mut self.machine, Vec::new())? else {
            return Err(stop("E-TYPE-MISMATCH", "`dump` gives a List[Text]"));
        };
        let mut out = String::new();
        for l in lines {
            let Data::Text(t) = l else { return Err(stop("E-TYPE-MISMATCH", "`dump` gives a List[Text]")) };
            out.push_str(&t);
            out.push('\n');
        }
        Ok(out)
    }

    pub fn report(&mut self) -> R<String> {
        match (self.shape.report.call)(&mut self.machine, Vec::new())? {
            Data::Text(t) => Ok(format!("{t}\n")),
            _ => Err(stop("E-TYPE-MISMATCH", "`report` gives Text")),
        }
    }
}
