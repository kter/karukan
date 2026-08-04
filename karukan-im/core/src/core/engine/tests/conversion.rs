use super::*;
use karukan_engine::{LearningCache, LearningConfig};

fn start_ai_conversion(engine: &mut InputMethodEngine) {
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    let result = engine.process_key(&press_key(Keysym::SPACE));
    assert!(result.consumed);
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
}

fn assert_segment_reading_invariant(engine: &InputMethodEngine) {
    let InputState::Conversion { segments, .. } = engine.state() else {
        panic!("expected Conversion state");
    };
    assert_eq!(
        segments
            .iter()
            .map(|segment| segment.reading.as_str())
            .collect::<String>(),
        engine.input_buf.settled_reading(&engine.converters.romaji)
    );
}

#[test]
fn test_conversion_char_commits_and_continues() {
    let mut engine = InputMethodEngine::new();

    // Type "あい" and enter conversion
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    // Type 'k' during conversion → should commit candidate and start new input
    let result = engine.process_key(&press('k'));
    assert!(result.consumed);

    // Should have committed the conversion
    let has_commit = result
        .actions
        .iter()
        .any(|a| matches!(a, EngineAction::Commit(_)));
    assert!(has_commit, "Should have a commit action");

    // Should now be in Composing with 'k' in preedit
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "k");
}

#[test]
fn test_conversion_char_commits_and_continues_romaji() {
    let mut engine = InputMethodEngine::new();

    // Type "あ" and enter conversion
    engine.process_key(&press('a'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    // Type 'k', 'a' → commits conversion, then starts "か"
    engine.process_key(&press('k'));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "k");

    engine.process_key(&press('a'));
    assert_eq!(engine.preedit().unwrap().text(), "か");
}

#[test]
fn test_alphabet_mode_space_inserts_literal_space() {
    let mut engine = InputMethodEngine::new();

    // Enter alphabet mode via Shift+N
    engine.process_key(&press_shift('N'));
    assert!(engine.mode.current() == InputMode::Alphabet);

    // Type "ew"
    engine.process_key(&press('e'));
    engine.process_key(&press('w'));
    assert_eq!(engine.preedit().unwrap().text(), "New");

    // Space → should insert literal space, NOT start conversion
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "New ");

    // Type "york"
    engine.process_key(&press('y'));
    engine.process_key(&press('o'));
    engine.process_key(&press('r'));
    engine.process_key(&press('k'));
    assert_eq!(engine.preedit().unwrap().text(), "New york");
}

#[test]
fn test_tab_navigation_in_conversion() {
    let mut engine = InputMethodEngine::new();

    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
    assert!(engine.state().candidates().unwrap().len() >= 2);
    assert_eq!(engine.state().candidates().unwrap().cursor(), 0);

    engine.process_key(&press_key(Keysym::TAB));
    assert_eq!(engine.state().candidates().unwrap().cursor(), 1);

    engine.process_key(&press_shift_key(Keysym::TAB));
    assert_eq!(engine.state().candidates().unwrap().cursor(), 0);

    engine.process_key(&press_key(Keysym::TAB));
    assert_eq!(engine.state().candidates().unwrap().cursor(), 1);

    engine.process_key(&press_key(Keysym::ISO_LEFT_TAB));
    assert_eq!(engine.state().candidates().unwrap().cursor(), 0);
}

#[test]
fn shift_arrows_create_and_delete_segments_at_boundaries() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);

    let InputState::Conversion {
        segments, focus, ..
    } = engine.state()
    else {
        unreachable!()
    };
    assert_eq!(segments.len(), 1);
    assert_eq!(*focus, 0);
    assert_eq!(segments[0].reading, "あい");
    assert_segment_reading_invariant(&engine);

    let result = engine.process_key(&press_shift_key(Keysym::RIGHT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());

    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    assert!(result.consumed);
    let InputState::Conversion { segments, .. } = engine.state() else {
        unreachable!()
    };
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].reading, "あ");
    assert_eq!(segments[1].reading, "い");
    assert_segment_reading_invariant(&engine);

    assert!(engine.process_key(&press_key(Keysym::RIGHT)).consumed);
    let result = engine.process_key(&press_shift_key(Keysym::RIGHT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    assert_segment_reading_invariant(&engine);
    assert!(engine.process_key(&press_key(Keysym::LEFT)).consumed);

    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    assert_eq!(
        match engine.state() {
            InputState::Conversion { segments, .. } => segments.len(),
            _ => 0,
        },
        2,
        "a one-character focused segment cannot shrink"
    );

    let result = engine.process_key(&press_shift_key(Keysym::RIGHT));
    assert!(result.consumed);
    let InputState::Conversion { segments, .. } = engine.state() else {
        unreachable!()
    };
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].reading, "あい");
    assert_segment_reading_invariant(&engine);
}

#[test]
fn conversion_preedit_attributes_track_all_segments_and_focus() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);

    let preedit = engine.preedit().unwrap();
    assert_eq!(preedit.attributes().len(), 1);
    assert_eq!(preedit.attributes()[0].start, 0);
    assert_eq!(preedit.attributes()[0].end, preedit.text().chars().count());
    assert_eq!(preedit.attributes()[0].attr_type, AttributeType::Highlight);

    engine.process_key(&press_shift_key(Keysym::LEFT));
    let preedit = engine.preedit().unwrap();
    let attributes = preedit.attributes();
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].start, 0);
    assert_eq!(attributes[0].end, preedit.caret());
    assert_eq!(attributes[0].attr_type, AttributeType::Highlight);
    assert_eq!(attributes[1].start, preedit.caret());
    assert_eq!(attributes[1].end, preedit.text().chars().count());
    assert_eq!(attributes[1].attr_type, AttributeType::Underline);

    engine.process_key(&press_key(Keysym::RIGHT));
    let preedit = engine.preedit().unwrap();
    let attributes = preedit.attributes();
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].attr_type, AttributeType::Underline);
    assert_eq!(attributes[1].attr_type, AttributeType::Highlight);
    assert_eq!(preedit.caret(), preedit.text().chars().count());
}

#[test]
fn preedit_has_one_attribute_per_segment_for_three_segments() {
    let mut engine = InputMethodEngine::new();
    for ch in ['a', 'i', 'u', 'e'] {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    engine.process_key(&press_shift_key(Keysym::LEFT));
    engine.process_key(&press_shift_key(Keysym::LEFT));
    engine.process_key(&press_key(Keysym::RIGHT));
    engine.process_key(&press_shift_key(Keysym::LEFT));

    let InputState::Conversion {
        segments, focus, ..
    } = engine.state()
    else {
        unreachable!()
    };
    assert_eq!(segments.len(), 3);
    assert_eq!(*focus, 1);
    let preedit = engine.preedit().unwrap();
    assert_eq!(preedit.attributes().len(), 3);
    assert_eq!(preedit.attributes()[0].attr_type, AttributeType::Underline);
    assert_eq!(preedit.attributes()[1].attr_type, AttributeType::Highlight);
    assert_eq!(preedit.attributes()[2].attr_type, AttributeType::Underline);
    assert_eq!(preedit.attributes()[0].start, 0);
    assert_eq!(preedit.attributes()[0].end, preedit.attributes()[1].start);
    assert_eq!(preedit.attributes()[1].end, preedit.attributes()[2].start);
    assert_eq!(preedit.attributes()[2].end, preedit.text().chars().count());
    assert_eq!(preedit.caret(), preedit.attributes()[1].end);
    assert_segment_reading_invariant(&engine);
}

#[test]
fn enter_commits_all_segments_and_records_learning_per_segment() {
    let mut engine = InputMethodEngine::new();
    engine.learning = Some(LearningCache::new(LearningConfig::default()));
    engine.set_surrounding_context("前", "");
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));
    let (expected, selections) = engine.selected_conversion_info().unwrap();

    let result = engine.process_key(&press_key(Keysym::RETURN));

    assert!(result.consumed);
    assert!(
        result
            .actions
            .iter()
            .any(|action| matches!(action, EngineAction::Commit(text) if text == &expected))
    );
    assert!(matches!(engine.state(), InputState::Empty));
    assert!(engine.input_buf.is_empty());
    assert_eq!(
        engine.surrounding_context.as_ref().unwrap().left.as_deref(),
        Some(format!("前{expected}").as_str())
    );

    let cache = engine.learning.as_ref().unwrap();
    for (reading, surface) in selections {
        assert!(
            cache
                .lookup(&reading)
                .iter()
                .any(|(text, _)| text == &surface)
        );
    }
    assert!(cache.lookup("あい").is_empty());
}

#[test]
fn committed_surrounding_context_is_bounded_and_keeps_latest_text() {
    let mut engine = InputMethodEngine::new();
    engine.config.display_context_len = 5;
    engine.config.max_api_context_len = 8;
    let context_limit = engine
        .config
        .display_context_len
        .max(engine.config.max_api_context_len);
    let committed = [
        "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "十一", "十二", "十三", "十四",
        "十五", "十六", "十七", "十八", "十九", "二十",
    ];

    for text in committed {
        engine.finish_conversion(text, &[("よみ".to_string(), text.to_string())]);
    }

    let left = engine
        .surrounding_context
        .as_ref()
        .and_then(|context| context.left.as_deref())
        .unwrap();
    assert!(left.chars().count() <= context_limit);
    assert!(left.ends_with(committed.last().unwrap()));
}

#[test]
fn digit_selection_moves_focus_without_committing() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));

    let result = engine.process_key(&press('1'));

    assert!(result.consumed);
    assert!(
        !result
            .actions
            .iter()
            .any(|action| matches!(action, EngineAction::Commit(_)))
    );
    let InputState::Conversion {
        focus, segments, ..
    } = engine.state()
    else {
        panic!("digit selection must keep Conversion state");
    };
    assert_eq!(*focus, 1);
    assert_eq!(segments.len(), 2);
    assert_eq!(engine.input_buf.reading(), "あい");
    assert_segment_reading_invariant(&engine);

    let result = engine.process_key(&press('1'));
    assert!(result.consumed);
    assert!(
        !result
            .actions
            .iter()
            .any(|action| matches!(action, EngineAction::Commit(_)))
    );
    assert!(matches!(
        engine.state(),
        InputState::Conversion { focus: 1, .. }
    ));
}

#[test]
fn escape_after_resize_restores_the_whole_reading() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));

    let result = engine.process_key(&press_key(Keysym::ESCAPE));

    assert!(result.consumed);
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.input_buf.reading(), "あい");
    assert_eq!(engine.preedit().unwrap().text(), "あい");
}

#[test]
fn plain_arrows_move_focus_and_are_consumed_at_boundaries() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));

    let result = engine.process_key(&press_key(Keysym::LEFT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    assert!(matches!(
        engine.state(),
        InputState::Conversion { focus: 0, .. }
    ));

    assert!(engine.process_key(&press_key(Keysym::RIGHT)).consumed);
    assert!(matches!(
        engine.state(),
        InputState::Conversion { focus: 1, .. }
    ));
    let result = engine.process_key(&press_key(Keysym::RIGHT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    assert!(engine.process_key(&press_key(Keysym::LEFT)).consumed);
    assert!(matches!(
        engine.state(),
        InputState::Conversion { focus: 0, .. }
    ));
    assert_segment_reading_invariant(&engine);
}

#[test]
fn aux_segment_position_only_appears_with_multiple_segments() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    let result = engine.process_key(&press_key(Keysym::SPACE));
    let full_aux = last_aux_text(&result).unwrap();
    assert!(!full_aux.contains("文節"));

    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    let shortened_aux = last_aux_text(&result).unwrap();
    assert!(
        shortened_aux.contains(" 1/2文節 "),
        "multi-segment conversion aux must show focus position: {shortened_aux}"
    );
}

#[test]
fn focus_out_commit_preserves_all_segments() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));
    let expected = engine.selected_conversion_info().unwrap().0;

    assert_eq!(engine.commit(), expected);
    assert!(matches!(engine.state(), InputState::Empty));
    assert!(engine.input_buf.is_empty());
}

#[test]
fn second_segment_context_contains_first_selected_surface() {
    let mut engine = InputMethodEngine::new();
    engine.set_surrounding_context("前", "");
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));
    engine.state.candidates_mut().unwrap().move_next();
    let first_surface = engine.state.candidates().unwrap().selected_text().unwrap();

    assert_eq!(engine.segment_lctx(1), format!("前{first_surface}"));
}
