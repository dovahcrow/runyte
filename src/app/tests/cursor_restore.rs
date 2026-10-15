// SPDX-License-Identifier: MPL-2.0

use super::*;

fn add_buffer(app: &mut App, contents: &str) -> usize {
    let mut buffer = Buffer::scratch();
    buffer.apply(&Transaction::insert(0, contents));
    let id = app.buffers.len();
    app.buffers.push(buffer);
    app.syntax.push(None);
    id
}

fn pick_buffer(app: &mut App, id: usize) {
    for ch in " bb".chars() {
        press(app, ch);
    }
    let picker = app.list.as_mut().unwrap();
    picker.selected = picker.visible_indices().iter().position(|index| {
        matches!(app.list_actions[picker.items[*index].index], ListAction::Destination(OpenDestination::Buffer(buffer)) if buffer == id)
    }).unwrap();
    key(app, KeyCode::Enter, Modifiers::NONE);
    assert_eq!(app.active().buffer, id);
}

#[test]
fn buffer_picker_restores_selection_and_viewport_on_each_visit() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "zero\none\ntwo\nthree\nfour\nfive");
    let other = add_buffer(&mut app, "alpha\nbeta\ngamma\ndelta");
    let original = Selection::new(vec![Range::new(11, 9), Range::point(21)], 1);
    app.replace_active_selection(original.clone());
    app.active_mut().scroll_row = 2;
    app.active_mut().scroll_wrap = 1;
    app.active_mut().scroll_col = 3;
    pick_buffer(&mut app, other);
    set_cursor(&mut app, 2, 2);
    let other_selection = app.active().selection.clone();
    pick_buffer(&mut app, 0);
    assert_eq!(app.active().selection, original);
    assert_eq!(
        (
            app.active().scroll_row,
            app.active().scroll_wrap,
            app.active().scroll_col
        ),
        (2, 1, 3)
    );
    pick_buffer(&mut app, other);
    assert_eq!(app.active().selection, other_selection);
}

#[test]
fn buffer_return_positions_are_per_pane_and_follow_shared_edits() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "abcdef\nsecond\nthird");
    let first = app.active_pane;
    app.replace_active_selection(Selection::point(10));
    app.split(Axis::Horizontal, None).unwrap();
    let second = app.active_pane;
    app.replace_active_selection(Selection::point(2));
    let other = add_buffer(&mut app, "other");
    app.switch_buffer(other);
    app.active_pane = first;
    app.edit(Transaction::insert(0, "XYZ"));
    assert_eq!(app.active().selection, Selection::point(13));
    app.active_pane = second;
    app.switch_buffer(0);
    assert_eq!(app.active().selection, Selection::point(5));
    app.switch_buffer(other);
    app.active_pane = first;
    press(&mut app, 'u');
    assert_eq!(app.active().selection, Selection::point(10));
    app.active_pane = second;
    app.switch_buffer(0);
    assert_eq!(app.active().selection, Selection::point(2));
}

#[test]
fn undo_redo_restore_checkpoint_selections_after_moving_elsewhere() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "alpha\nbeta\ngamma\ndelta");
    set_cursor(&mut app, 1, 2);
    let before = app.active().selection.clone();
    press(&mut app, 'i');
    for ch in "XYZ".chars() {
        press(&mut app, ch);
    }
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    let after = app.active().selection.clone();
    set_cursor(&mut app, 3, 3);
    press(&mut app, 'u');
    assert_eq!(text(&app), "alpha\nbeta\ngamma\ndelta");
    assert_eq!(app.active().selection, before);
    set_cursor(&mut app, 0, 0);
    press(&mut app, 'U');
    assert_eq!(text(&app), "alpha\nbeXYZta\ngamma\ndelta");
    assert_eq!(app.active().selection, after);
    set_cursor(&mut app, 3, 3);
    press(&mut app, 'u');
    assert_eq!(app.active().selection, before);
}

#[test]
fn deletion_undo_restores_all_ranges_and_primary_selection_after_buffer_visit() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "abc def ghi\nother");
    let before = Selection::new(vec![Range::new(2, 0), Range::new(4, 6)], 1);
    app.replace_active_selection(before.clone());
    press(&mut app, 'd');
    let after = app.active().selection.clone();
    assert_eq!(text(&app), "  ghi\nother");
    let other = add_buffer(&mut app, "notes");
    pick_buffer(&mut app, other);
    pick_buffer(&mut app, 0);
    set_cursor(&mut app, 1, 2);
    press(&mut app, 'u');
    assert_eq!(text(&app), "abc def ghi\nother");
    assert_eq!(app.active().selection, before);
    set_cursor(&mut app, 1, 0);
    press(&mut app, 'U');
    assert_eq!(app.active().selection, after);
}

#[test]
fn undo_restores_only_the_invoking_pane_and_branching_discards_redo() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "abcdef\nsecond");
    set_cursor(&mut app, 0, 2);
    press(&mut app, 'i');
    press(&mut app, 'X');
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    app.split(Axis::Horizontal, None).unwrap();
    let second = app.active_pane;
    app.replace_active_selection(Selection::point(11));
    app.active_pane = 0;
    set_cursor(&mut app, 1, 0);
    press(&mut app, 'u');
    assert_eq!(app.active().selection, Selection::point(2));
    assert_eq!(app.panes[&second].selection, Selection::point(10));
    press(&mut app, 'r');
    press(&mut app, 'Q');
    press(&mut app, 'U');
    assert_eq!(app.status, "nothing to redo");
    press(&mut app, 'u');
    assert_eq!(text(&app), "abcdef\nsecond");
    assert_eq!(app.active().selection, Selection::point(2));
}

#[test]
fn opening_an_existing_file_preserves_its_last_position() {
    let path = temporary("cursor-return.txt");
    fs::write(&path, "alpha\nbeta\ngamma").unwrap();
    let mut app = App::new(Config::default(), Some(path.clone())).unwrap();
    set_cursor(&mut app, 1, 2);
    app.open_file(path.clone()).unwrap();
    assert_eq!(cursor(&app), Position::new(1, 2));
    let other = add_buffer(&mut app, "notes");
    app.switch_buffer(other);
    app.open_file(path.clone()).unwrap();
    assert_eq!(cursor(&app), Position::new(1, 2));
    fs::remove_file(path).unwrap();
}

#[test]
fn empty_insert_session_does_not_change_the_previous_redo_position() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "alpha\nbeta\ngamma");
    set_cursor(&mut app, 1, 1);
    press(&mut app, 'i');
    press(&mut app, 'X');
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    let after = app.active().selection.clone();
    set_cursor(&mut app, 2, 2);
    press(&mut app, 'i');
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    press(&mut app, 'u');
    assert_eq!(cursor(&app), Position::new(1, 1));
    press(&mut app, 'U');
    assert_eq!(app.active().selection, after);
}

#[test]
fn returning_and_undoing_preserve_selection_semantics() {
    let mut config = Config::default();
    config.editor.grammar = GrammarKind::Vim;
    let mut app = App::new(config, None).unwrap();
    seed(&mut app, "alpha\nbeta\ngamma");
    app.replace_active_selection(Selection::new(vec![Range::new(6, 11)], 0));
    app.active_mut()
        .mark_selection_semantics(SelectionSemantics::VimLinewise);
    let before = app.active().selection.clone();
    let other = add_buffer(&mut app, "other");
    app.switch_buffer(other);
    app.switch_buffer(0);
    assert_eq!(app.active().selection, before);
    assert_eq!(
        app.active().selection_semantics(),
        SelectionSemantics::VimLinewise
    );
    app.execute_editor_command(EditorCommand::DeleteSelection)
        .unwrap();
    assert_eq!(text(&app), "alpha\ngamma");
    let after = app.active().selection.clone();
    let after_semantics = app.active().selection_semantics();
    press(&mut app, 'u');
    assert_eq!(text(&app), "alpha\nbeta\ngamma");
    assert_eq!(app.active().selection, before);
    assert_eq!(
        app.active().selection_semantics(),
        SelectionSemantics::VimLinewise
    );
    app.redo();
    assert_eq!(app.active().selection, after);
    assert_eq!(app.active().selection_semantics(), after_semantics);
}

#[test]
fn buffer_return_positions_are_bounded_and_removed_on_close() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "original");
    for _ in 0..140 {
        let id = add_buffer(&mut app, "contents");
        app.switch_buffer(id);
        assert!(app.active().saved_view_positions.len() <= 128);
    }
    let last = app.active().buffer;
    let other = add_buffer(&mut app, "other");
    app.switch_buffer(other);
    assert!(app.active().saved_view_positions.contains_key(&last));
    app.switch_buffer(last);
    app.execute_command("close!").unwrap();
    assert!(!app.active().saved_view_positions.contains_key(&last));
}

#[test]
fn open_line_undo_and_redo_restore_their_distinct_positions() {
    let mut app = App::new(Config::default(), None).unwrap();
    seed(&mut app, "first\nsecond\nthird");
    set_cursor(&mut app, 1, 3);
    press(&mut app, 'o');
    press(&mut app, 'X');
    key(&mut app, KeyCode::Escape, Modifiers::NONE);
    let after = app.active().selection.clone();
    set_cursor(&mut app, 0, 0);
    press(&mut app, 'u');
    assert_eq!(cursor(&app), Position::new(1, 3));
    assert_eq!(text(&app), "first\nsecond\nthird");
    press(&mut app, 'U');
    assert_eq!(text(&app), "first\nsecond\nX\nthird");
    assert_eq!(app.active().selection, after);
}
