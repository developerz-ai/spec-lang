// Projected by hand from note @store: "Bytes written" (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use crate::board::World;
use crate::model::Task;

const HEAD: &str = "{\"version\":1,\"next_no\":";
/// The longest fixed part of one task, the comma after it and a 16-digit number included.
const TASK_FRAME: usize =
    "{\"no\":9007199254740992,\"title\":\"\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]},".len();

/// The one canonical form: one line, keys in order, tasks by number, tags by
/// code point, only `"` and `\` escaped, everything else raw UTF-8.
/// The world's maps and sets already iterate in those orders, so nothing is sorted here.
pub fn encode(world: &World) -> Vec<u8> {
    let mut text = String::with_capacity(capacity(world));
    text.push_str(HEAD);
    push_number(&mut text, world.next_no().get());
    text.push_str(",\"tasks\":[");
    for (index, task) in world.tasks().values().enumerate() {
        if index > 0 {
            text.push(',');
        }
        push_task(&mut text, task);
    }
    text.push_str("]}\n");
    text.into_bytes()
}

/// An upper bound on the encoded size, so the buffer is allocated once and handed to one `write`.
fn capacity(world: &World) -> usize {
    let tasks: usize = world
        .tasks()
        .values()
        .map(|task| {
            let tags: usize = task.tags.iter().map(|tag| tag.as_str().len() + 3).sum();
            // Twice the title: every byte of it may need a backslash.
            TASK_FRAME + 2 * task.title.as_str().len() + tags
        })
        .sum();
    HEAD.len() + 16 + ",\"tasks\":[]}\n".len() + tasks
}

fn push_task(text: &mut String, task: &Task) {
    text.push_str("{\"no\":");
    push_number(text, task.no.get());
    text.push_str(",\"title\":");
    push_string(text, task.title.as_str());
    text.push_str(",\"status\":\"");
    text.push_str(task.status.name());
    text.push_str("\",\"priority\":\"");
    text.push_str(task.priority.name());
    text.push_str("\",\"tags\":[");
    for (index, tag) in task.tags.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        // A Tag is lower-case letters, digits and hyphens: nothing in it is ever escaped.
        text.push('"');
        text.push_str(tag.as_str());
        text.push('"');
    }
    text.push_str("]}");
}

/// A Title has no byte below 32, so `"` and `\` are the only escapes JSON needs.
fn push_string(text: &mut String, value: &str) {
    text.push('"');
    let mut rest = value;
    while let Some(at) = rest.find(['"', '\\']) {
        let (plain, special) = rest.split_at(at);
        text.push_str(plain);
        text.push('\\');
        let mut chars = special.chars();
        if let Some(c) = chars.next() {
            text.push(c);
        }
        rest = chars.as_str();
    }
    text.push_str(rest);
    text.push('"');
}

fn push_number(text: &mut String, value: u64) {
    let mut digits = [0u8; 20];
    let mut count = 0;
    let mut rest = value;
    loop {
        // `rest % 10` is below 10, so the cast keeps every bit.
        digits[count] = b'0' + (rest % 10) as u8;
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    for &digit in digits[..count].iter().rev() {
        text.push(char::from(digit));
    }
}
