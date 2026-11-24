//! Spec names to target names. A name comes from the declaration alone, so
//! adding a declaration never renames its neighbours.

fn words(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c == '_' || c == '-' || c == '.' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn capital(w: &str) -> String {
    let mut cs = w.chars();
    match cs.next() {
        Some(c) => c.to_uppercase().chain(cs).collect(),
        None => String::new(),
    }
}

/// `next_no` gives `nextNo`.
pub fn camel(name: &str) -> String {
    let ws = words(name);
    ws.iter().enumerate().map(|(i, w)| if i == 0 { w.clone() } else { capital(w) }).collect()
}

/// `next_no` gives `NextNo`.
pub fn pascal(name: &str) -> String {
    words(name).iter().map(|w| capital(w)).collect()
}

/// `TaskNo` gives `task-no`.
pub fn kebab(name: &str) -> String {
    words(name).join("-")
}

/// `TaskNo` gives `task_no`.
pub fn snake(name: &str) -> String {
    words(name).join("_")
}

/// `TaskNo` gives `TASK_NO`.
pub fn screaming(name: &str) -> String {
    snake(name).to_uppercase()
}

/// The path of a module's source, from its name: the path is the module.
pub fn slg_path(module: &str) -> String {
    format!("{}.slg", module.replace('.', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases() {
        assert_eq!(camel("next_no"), "nextNo");
        assert_eq!(camel("is_title"), "isTitle");
        assert_eq!(pascal("three_open_one_done"), "ThreeOpenOneDone");
        assert_eq!(kebab("TaskNo"), "task-no");
        assert_eq!(snake("TaskNo"), "task_no");
        assert_eq!(screaming("Priority"), "PRIORITY");
        assert_eq!(slg_path("todo.board"), "todo/board.slg");
    }
}
