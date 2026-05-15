use std::sync::LazyLock;

use regex::Regex;

static TEST_COMMANDS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"^\s*bun\s+test\b",
        r"^\s*cargo\s+test\b",
        r"^\s*npm\s+test\b",
        r"^\s*npx\s+jest\b",
        r"^\s*npx\s+vitest\b",
        r"^\s*pytest\b",
        r"^\s*go\s+test\b",
    ]
    .into_iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

pub fn is_test_command(command: &str) -> bool {
    TEST_COMMANDS.iter().any(|p| p.is_match(command))
}
