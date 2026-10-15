// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::jump_labels::LabelPart;

fn start(app: &mut App) {
    for character in "gw".chars() {
        press(app, character);
    }
}

fn query(app: &mut App, pair: &str) {
    start(app);
    for character in pair.chars() {
        press(app, character);
    }
}

#[test]
fn leap_waits_for_two_characters_and_a_label_then_records_a_jump() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "alpha beta\nx alpha");
    app.active_mut().wrap_width = 80;
    start(&mut app);
    assert!(app.jump.as_ref().unwrap().is_empty());
    press(&mut app, 'p');
    assert!(app.jump.as_ref().unwrap().is_empty());
    assert!(app.status.contains("second character"));
    press(&mut app, 'h');
    assert_eq!(cursor(&app), Position::default());
    let labels = app.jump.as_ref().unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(labels.label_at(2), Some(('a', LabelPart::Immediate)));
    assert_eq!(labels.label_at(15), Some(('s', LabelPart::Immediate)));
    press(&mut app, 's');
    assert!(app.jump.is_none());
    assert_eq!(cursor(&app), Position::new(1, 4));
    key(&mut app, KeyCode::Char('o'), Modifiers::CONTROL);
    assert_eq!(cursor(&app), Position::default());
    key(&mut app, KeyCode::Char('i'), Modifiers::CONTROL);
    assert_eq!(cursor(&app), Position::new(1, 4));
    assert_eq!(text(&app), "alpha beta\nx alpha");
}

#[test]
fn leap_matches_case_insensitive_pairs_including_spaces_and_punctuation() {
    for (pair, offsets) in [
        ("::", vec![1, 7]),
        (" a", vec![3, 9, 12]),
        ("Ab", vec![4, 10]),
        ("ab", vec![4, 10]),
        ("aa", vec![13, 14]),
    ] {
        let mut app = App::new(Config::default(), None).unwrap();
        seed(&mut app, "x:: ab :: Ab aaa");
        app.active_mut().wrap_width = 80;
        query(&mut app, pair);
        let labels = app.jump.as_ref().unwrap();
        assert_eq!(labels.len(), offsets.len(), "{pair}");
        for offset in offsets {
            assert!(labels.label_at(offset).is_some(), "{pair}: {offset}");
        }
    }
}

#[test]
fn leap_ignores_shift_for_symbols_and_unicode_letter_case() {
    for (plain, shifted) in [
        ('`', '~'),
        ('1', '!'),
        ('2', '@'),
        ('3', '#'),
        ('4', '$'),
        ('5', '%'),
        ('6', '^'),
        ('7', '&'),
        ('8', '*'),
        ('9', '('),
        ('0', ')'),
        ('-', '_'),
        ('=', '+'),
        ('[', '{'),
        (']', '}'),
        ('\\', '|'),
        (';', ':'),
        ('\'', '"'),
        (',', '<'),
        ('.', '>'),
        ('/', '?'),
        ('é', 'É'),
        ('ж', 'Ж'),
    ] {
        for first in [plain, shifted] {
            let mut app = App::new(Config::default(), None).unwrap();
            seed(&mut app, &format!("{plain}x {shifted}X"));
            app.active_mut().wrap_width = 80;
            query(&mut app, &format!("{first}X"));
            let labels = app.jump.as_ref().unwrap();
            assert_eq!(labels.len(), 2, "{first}");
            assert!(labels.label_at(0).is_some());
            assert!(labels.label_at(3).is_some());
            press(&mut app, 's');
            assert_eq!(cursor(&app), Position::new(0, 3));
        }
    }
}

#[test]
fn leap_bracket_equivalence_keeps_different_keys_distinct() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "[] {} ()");
    app.active_mut().wrap_width = 80;
    query(&mut app, "{}");
    let labels = app.jump.as_ref().unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.label_at(0).is_some());
    assert!(labels.label_at(3).is_some());
    assert_eq!(labels.label_at(6), None);
}

#[test]
fn leap_jumps_backward_and_extends_select_mode_without_editing() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "alpha\nbeta alpha");
    app.active_mut().wrap_width = 80;
    set_cursor(&mut app, 1, 9);
    press(&mut app, 'v');
    let anchor = app.active().selection.primary().anchor;
    query(&mut app, "ph");
    press(&mut app, 's');
    assert_eq!(app.mode, Mode::Select);
    assert_eq!(app.active().selection.primary().anchor, anchor);
    assert_eq!(cursor(&app), Position::new(0, 2));
}

#[test]
fn leap_cancellation_and_missing_targets_consume_input_without_motion_or_edit() {
    for prefix in ["", "a", "al"] {
        for (code, modifiers) in [
            (KeyCode::Escape, Modifiers::NONE),
            (KeyCode::Left, Modifiers::NONE),
            (KeyCode::Char('c'), Modifiers::CONTROL),
        ] {
            let mut app = App::new(Config::default(), None).unwrap();
            seed(&mut app, "alpha");
            app.active_mut().wrap_width = 80;
            start(&mut app);
            for character in prefix.chars() {
                press(&mut app, character);
            }
            key(&mut app, code, modifiers);
            assert!(app.jump.is_none());
            assert_eq!(cursor(&app), Position::default());
            assert_eq!(app.mode, Mode::Normal);
            assert_eq!(text(&app), "alpha");
        }
    }
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "alpha");
    app.active_mut().wrap_width = 80;
    query(&mut app, "zz");
    assert!(app.jump.is_none());
    assert!(app.status_error);
    query(&mut app, "al");
    press(&mut app, 'i');
    assert!(app.jump.is_none());
    assert!(app.status_error);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn leap_respects_horizontal_scroll_tabs_wide_cells_and_soft_wrap_segments() {
    for (text, scroll, width, pair, expected) in [
        ("zz zz aa aa aa", 6, 5, "aa", vec![6, 9]),
        ("界界 aa aa", 0, 7, "aa", vec![3]),
        ("\t\t aa aa", 0, 11, "aa", vec![3]),
        ("界界 中文 中文", 0, 12, "中文", vec![3, 6]),
        ("xaaaa", 0, 4, "aa", vec![1, 2, 3]),
    ] {
        let mut app = App::new(Config::default(), None).unwrap();
        app.config.editor.soft_wrap = false;
        seed(&mut app, text);
        app.active_mut().scroll_col = scroll;
        app.active_mut().wrap_width = width;
        query(&mut app, pair);
        let labels = app.jump.as_ref().unwrap();
        assert_eq!(labels.len(), expected.len(), "{text}");
        for offset in expected {
            assert!(labels.label_at(offset).is_some(), "{text}: {offset}");
        }
    }

    let mut app = App::new(Config::default(), None).unwrap();
    app.config.editor.soft_wrap = true;
    seed(&mut app, &format!("{}tail", "aa ".repeat(50)));
    app.active_mut().wrap_width = 6;
    app.active_mut().scroll_wrap = 1;
    query(&mut app, "aa");
    let labels = app.jump.as_ref().unwrap();
    assert_eq!(labels.label_at(0), None);
    assert!(labels.label_at(6).is_some());
    assert_eq!(labels.label_at(147), None);
}

#[test]
fn leap_many_matches_narrow_labels_and_overlaps_do_not_hide_actionable_keys() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, &"aa ".repeat(27));
    app.active_mut().wrap_width = 100;
    query(&mut app, "aa");
    assert_eq!(app.jump.as_ref().unwrap().len(), 27);
    press(&mut app, 'm');
    assert_eq!(cursor(&app), Position::default());
    press(&mut app, 's');
    assert_eq!(cursor(&app), Position::new(0, 78));

    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, &"aA".repeat(20));
    app.active_mut().wrap_width = 100;
    set_cursor(&mut app, 0, 0);
    query(&mut app, "aa");
    let labels = app.jump.as_ref().unwrap();
    assert!(!labels.is_empty());
    for offset in 0..40 {
        if let Some((_, part)) = labels.label_at(offset) {
            assert_eq!(part, LabelPart::Immediate);
        }
    }
}

#[test]
fn leap_is_independent_of_the_configured_leader_and_waits_for_a_single_match() {
    let config: Config = serde_yaml::from_str("keys:\n  leader: Ctrl-x\n").unwrap();
    let mut app = App::new(config, None).unwrap();
    seed(&mut app, "alpha beta");
    app.active_mut().wrap_width = 80;
    key(&mut app, KeyCode::Char('x'), Modifiers::CONTROL);
    assert_eq!(app.pending_sequence().to_string(), "Ctrl-x");
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    for character in "gwet".chars() {
        press(&mut app, character);
    }
    assert_eq!(app.jump.as_ref().unwrap().len(), 1);
    assert_eq!(cursor(&app), Position::default());
    press(&mut app, 'a');
    assert_eq!(cursor(&app), Position::new(0, 7));
}

#[test]
fn leap_excludes_folded_text_and_does_not_skip_document_line_breaks() {
    let path = temporary("leap-fold.rs");
    fs::write(&path, "fn alpha() {\n    alpha();\n}\nalpha();\n").unwrap();
    let mut app = App::new(Config::default(), Some(path.clone())).unwrap();
    app.active_mut().wrap_width = 80;
    for character in " xf".chars() {
        press(&mut app, character);
    }
    assert!(!app.active().folds.collapsed.is_empty());
    query(&mut app, "ph");
    let labels = app.jump.as_ref().unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.label_at(5).is_some());
    assert_eq!(labels.label_at(app.buffers[0].line_to_offset(1) + 6), None);
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    query(&mut app, "}a");
    assert!(app.jump.is_none());
    assert!(app.status_error);
    fs::remove_file(path).unwrap();
}

#[test]
fn leap_space_matches_real_line_endings_as_well_as_literal_spaces() {
    for ending in ["\n", "\r\n"] {
        for soft_wrap in [false, true] {
            let mut app = App::new(Config::default(), None).unwrap();
            app.config.editor.soft_wrap = soft_wrap;
            let content = format!("a,{ending} b, {ending}last,");
            seed(&mut app, &content);
            app.active_mut().wrap_width = 80;
            query(&mut app, ", ");
            let labels = app.jump.as_ref().unwrap();
            assert_eq!(labels.len(), 2);
            assert_eq!(labels.label_at(1), Some(('a', LabelPart::Immediate)));
            assert_eq!(
                labels.label_at(app.buffers[0].line_to_offset(1) + 2),
                Some(('s', LabelPart::Immediate))
            );
            assert_eq!(labels.label_at(app.buffers[0].len_chars() - 1), None);
            press(&mut app, 'a');
            assert_eq!(cursor(&app), Position::new(0, 1));
            assert_eq!(text(&app), content);
        }
    }
}

#[test]
fn leap_line_end_matches_fit_the_last_cell_and_only_label_visible_segments() {
    for soft_wrap in [false, true] {
        let mut app = App::new(Config::default(), None).unwrap();
        app.config.editor.soft_wrap = soft_wrap;
        seed(&mut app, "a,\nb,");
        app.active_mut().wrap_width = 2;
        query(&mut app, ", ");
        assert_eq!(
            app.jump.as_ref().unwrap().label_at(1),
            Some(('a', LabelPart::Immediate))
        );
        press(&mut app, 'a');
        assert_eq!(cursor(&app), Position::new(0, 1));
    }

    let mut app = App::new(Config::default(), None).unwrap();
    app.config.editor.soft_wrap = true;
    seed(&mut app, "aa bb cc,\n");
    app.active_mut().wrap_width = 3;
    app.areas.insert(
        0,
        Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 3,
        },
    );
    query(&mut app, ", ");
    assert!(app.jump.is_none());
    assert!(app.status_error);
    app.active_mut().scroll_wrap = 2;
    query(&mut app, ", ");
    assert_eq!(app.jump.as_ref().unwrap().len(), 1);
}
