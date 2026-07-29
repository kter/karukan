use super::*;
use karukan_engine::{LearningCache, LearningConfig};

fn start_ai_conversion(engine: &mut InputMethodEngine) {
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    let result = engine.process_key(&press_key(Keysym::SPACE));
    assert!(result.consumed);
    assert!(matches!(engine.state(), InputState::Conversion { .. }));
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
fn shift_arrows_resize_conversion_target_with_clamped_boundaries() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);

    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 2, .. }
    ));

    let result = engine.process_key(&press_shift_key(Keysym::RIGHT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 2, .. }
    ));

    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    assert!(result.consumed);
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 1, .. }
    ));
    assert_eq!(engine.input_buf.text, "あい");

    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    assert!(result.consumed);
    assert!(result.actions.is_empty());
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 1, .. }
    ));

    let result = engine.process_key(&press_shift_key(Keysym::RIGHT));
    assert!(result.consumed);
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 2, .. }
    ));
}

#[test]
fn conversion_preedit_attributes_distinguish_target_and_remainder() {
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
    assert!(preedit.text().ends_with('い'));
}

#[test]
fn partial_enter_commits_target_then_converts_remainder() {
    let mut engine = InputMethodEngine::new();
    engine.set_surrounding_context("前", "");
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));
    let first_surface = engine
        .state()
        .candidates()
        .unwrap()
        .selected_text()
        .unwrap()
        .to_string();

    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert!(result.consumed);
    assert!(matches!(
        result.actions.first(),
        Some(EngineAction::Commit(text)) if text == &first_surface
    ));
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 1, .. }
    ));
    assert_eq!(engine.input_buf.text, "い");
    assert_eq!(
        engine.surrounding_context.as_ref().unwrap().left.as_deref(),
        Some(format!("前{first_surface}").as_str())
    );

    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert!(
        result
            .actions
            .iter()
            .any(|action| matches!(action, EngineAction::Commit(_)))
    );
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn partial_enter_records_learning_under_target_reading() {
    let mut engine = InputMethodEngine::new();
    engine.learning = Some(LearningCache::new(LearningConfig::default()));
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));
    let surface = engine
        .state()
        .candidates()
        .unwrap()
        .selected_text()
        .unwrap()
        .to_string();

    engine.process_key(&press_key(Keysym::RETURN));

    let cache = engine.learning.as_ref().unwrap();
    assert!(cache.lookup("あ").iter().any(|(text, _)| text == &surface));
    assert!(cache.lookup("あい").is_empty());
}

#[test]
fn digit_selection_uses_progressive_commit_path() {
    let mut engine = InputMethodEngine::new();
    engine.learning = Some(LearningCache::new(LearningConfig::default()));
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));
    let first_surface = engine
        .state()
        .candidates()
        .unwrap()
        .selected_text()
        .unwrap()
        .to_string();

    let result = engine.process_key(&press('1'));

    assert!(matches!(
        result.actions.first(),
        Some(EngineAction::Commit(text)) if text == &first_surface
    ));
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 1, .. }
    ));
    assert_eq!(engine.input_buf.text, "い");
    let cache = engine.learning.as_ref().unwrap();
    assert!(
        cache
            .lookup("あ")
            .iter()
            .any(|(text, _)| text == &first_surface)
    );
    assert!(cache.lookup("あい").is_empty());
}

#[test]
fn escape_after_resize_restores_the_whole_reading() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);
    engine.process_key(&press_shift_key(Keysym::LEFT));

    let result = engine.process_key(&press_key(Keysym::ESCAPE));

    assert!(result.consumed);
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.input_buf.text, "あい");
    assert_eq!(engine.preedit().unwrap().text(), "あい");
}

#[test]
fn plain_arrows_remain_unconsumed_during_conversion() {
    let mut engine = InputMethodEngine::new();
    start_ai_conversion(&mut engine);

    assert!(!engine.process_key(&press_key(Keysym::LEFT)).consumed);
    assert!(!engine.process_key(&press_key(Keysym::RIGHT)).consumed);
    assert!(matches!(
        engine.state(),
        InputState::Conversion { target_len: 2, .. }
    ));
}

#[test]
fn aux_target_position_only_appears_for_shortened_target() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    let result = engine.process_key(&press_key(Keysym::SPACE));
    let full_aux = last_aux_text(&result).unwrap();
    assert!(!full_aux.contains(" 2/2 あい"));

    let result = engine.process_key(&press_shift_key(Keysym::LEFT));
    let shortened_aux = last_aux_text(&result).unwrap();
    assert!(
        shortened_aux.contains(" 1/2 "),
        "shortened conversion aux must show target position: {shortened_aux}"
    );
}
