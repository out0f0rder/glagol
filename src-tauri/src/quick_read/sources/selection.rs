//! Bounded selection search along one ancestry chain; never scrape page contents.
pub(super) enum Probe {
    Missing,
    Boundary,
    Text(String),
}
const MAX_NODES: usize = 16;

/// Keep the focused element first, then try its text containers in order.
pub(super) fn first_selection<N>(
    start: N,
    mut inspect: impl FnMut(&N) -> Result<Probe, String>,
    mut parent: impl FnMut(&N) -> Option<N>,
) -> Result<Option<String>, String> {
    let mut node = start;
    for _ in 0..MAX_NODES {
        match inspect(&node)? {
            Probe::Text(text) => return Ok(Some(text)),
            Probe::Boundary => return Ok(None),
            Probe::Missing => {}
        }
        let Some(next) = parent(&node) else {
            break;
        };
        node = next;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn focus_without_text_pattern_reaches_selected_document() {
        let mut visited = Vec::new();
        let text = first_selection(
            0,
            |node| {
                visited.push(*node);
                Ok(if *node == 7 {
                    Probe::Text("Только выделение. Selected text.".into())
                } else {
                    Probe::Missing
                })
            },
            |node| Some(node + 1),
        )
        .unwrap();
        assert_eq!(text.as_deref(), Some("Только выделение. Selected text."));
        assert_eq!(visited, (0..=7).collect::<Vec<_>>());
    }

    #[test]
    fn collapsed_ranges_and_window_boundary_do_not_read_other_documents() {
        let parent_calls = Cell::new(0);
        let result = first_selection(
            0,
            |node| {
                Ok(if *node == 3 {
                    Probe::Boundary
                } else if *node > 3 {
                    Probe::Text("unrelated document".into())
                } else {
                    Probe::Missing
                })
            },
            |node| {
                parent_calls.set(parent_calls.get() + 1);
                Some(node + 1)
            },
        )
        .unwrap();
        assert!(result.is_none());
        assert_eq!(parent_calls.get(), 3);
    }

    #[test]
    fn protected_or_oversized_selection_cannot_fall_through() {
        for error in ["protected", "limit"] {
            let parent_calls = Cell::new(0);
            assert_eq!(
                first_selection(
                    0,
                    |_| Err(error.into()),
                    |_| {
                        parent_calls.set(parent_calls.get() + 1);
                        Some(1)
                    }
                )
                .unwrap_err(),
                error
            );
            assert_eq!(parent_calls.get(), 0);
        }
    }

    #[test]
    fn provider_cycle_has_a_fixed_budget() {
        let calls = Cell::new(0);
        assert!(first_selection(
            0,
            |_| {
                calls.set(calls.get() + 1);
                Ok(Probe::Missing)
            },
            |_| Some(0)
        )
        .unwrap()
        .is_none());
        assert_eq!(calls.get(), MAX_NODES);
    }
}
