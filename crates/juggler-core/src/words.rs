//! Words a screen puts together, made one way everywhere they're said.

/// Names as a list in words: "a", "a and b", "a, b and c"; nothing for none.
#[must_use]
pub fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::and_list;

    #[test]
    fn names_read_as_a_list() {
        let names = |n: &[&str]| n.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(and_list(&names(&[])), "");
        assert_eq!(and_list(&names(&["a"])), "a");
        assert_eq!(and_list(&names(&["a", "b"])), "a and b");
        assert_eq!(and_list(&names(&["a", "b", "c"])), "a, b and c");
    }
}
