use std::collections::VecDeque;

/// Removes leading indentation and leading/trailing line endings
pub fn dedent(text: &str) -> String {
    let mut smallest_indent: Option<usize> = None;

    // Find the line with the least amount of indentation
    for line in text.lines() {
        let mut line_indent = 0;

        // If the line only contains whitespace, ignore it
        if line.trim() == "" {
            continue;
        }

        for c in line.chars() {
            if c != ' ' {
                break;
            }

            line_indent += 1;
        }

        match smallest_indent {
            Some(x) => {
                if line_indent < x {
                    smallest_indent = Some(line_indent);
                }
            }
            None => smallest_indent = Some(line_indent),
        }
    }

    let mut lines: VecDeque<&str> = VecDeque::from([]);

    // If a common smallest indentation was found, a new string is built with the indentation
    // stripped
    if let Some(smallest_indent) = smallest_indent {
        for line in text.lines() {
            let stripped_line = line.get(smallest_indent..);

            lines.push_back(stripped_line.unwrap_or("").trim_end());
        }
    }

    loop {
        // Pop all leading empty lines
        if let Some(first_line) = lines.front() {
            if first_line.is_empty() {
                lines.pop_front();
                continue;
            }
        }

        // Pop all trailing empty lines
        if let Some(last_line) = lines.back() {
            if last_line.is_empty() {
                lines.pop_back();
                continue;
            }
        }

        break;
    }

    format!("{}\n", Vec::from(lines).join("\n"))
}
