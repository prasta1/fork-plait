pub(crate) mod a11y;
mod actions;
mod admission;
mod appearance;
mod branches;
mod build_output;
mod changes;
mod checkout_queue;
mod cleanup;
mod commands;
mod commit_panel;
mod commits;
mod composer;
mod conversation;
mod conversation_find;
mod draft_tabs;
mod draft_titles;
mod file_view;
mod harnesses;
mod history;
mod host;
mod images;
mod jump;
mod live_conversation;
mod markdown;
mod model_picker;
mod navigation;
mod navigation_host;
mod new_thread;
mod new_thread_view;
mod palette;
mod panels;
mod permissions;
mod selection;
mod session;
mod settings;
mod settings_agent;
mod settings_editor;
mod settings_isolation;
mod settings_keyboard;
mod settings_modal;
mod settings_policy;
mod settings_project;
mod settings_providers;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod snapshot;
mod tab_bookmark;
mod tab_identity;
mod tab_lifecycle;
mod tab_persistence;
mod tabs;
mod text_field;
pub mod theme;
mod thread_controls;
mod thread_files;
mod thread_workspace;
mod tool_card;
mod turn_records;
mod turn_review;
mod update;
mod ways_out;
mod window_bar;
mod worktree;
mod worktrees_view;

use gpui::{prelude::*, *};
use plait_core::{
    host_state::HostState,
    model::DiffMode,
    state::{AppState, Load},
};
use std::{path::PathBuf, time::Instant};
use theme::*;
pub use window_bar::TRAFFIC_LIGHTS;

actions!(
    plait,
    [
        Quit,
        Close,
        Minimize,
        OpenRepository,
        Settings,
        ProviderSettings,
        ProjectSettings,
        CheckForUpdates,
        Refresh,
        ToggleSidebar,
        ToggleDiagnostics,
        Conversation,
        Changes,
        CommitHistory,
        ShowFiles,
        QuickOpen,
        FindInFile,
        FindNext,
        FindPrevious,
        StageAll,
        UnstageAll,
        CommitStaged,
        Push,
        Pull,
        Fetch,
        SwitchBranch,
        NewBranch,
        NextThread,
        PreviousThread,
        UndoLastTurn,
        Tab,
        BackTab,
        NextFile,
        PreviousFile,
        NewThread,
        NewThreadWithModel,
        NewWorktreeThread,
        SwitchThreadModel,
        CommandPalette,
        ToggleInspector,
        ToggleChangedFiles,
        RenameThread,
        ArchiveThread,
        ToggleArchived,
        FilterThreads,
        ToggleNotifications,
        DenyOnce,
        AllowOnce
    ]
);

/// The workspace beside the sidebar (presentation-independent view kind from core).
pub(super) use plait_core::thread_view::ThreadView as Workspace;

/// Below this width the Git inspector is not shown; Git stays in Changes.
const WIDE_WINDOW: f32 = 1150.;

pub struct Plait {
    tabs: tabs::Tabs,
    state: AppState,
    host: HostState,
    navigation: navigation::Navigation,
    /// Presentation state for the single selected thread workspace.
    workspace: thread_workspace::ThreadWorkspace,
    commit_sends:
        std::collections::HashMap<plait_core::state::CheckoutId, commit_panel::CommitAttempt>,
    /// Window modal, independent of the thread workspace.
    settings_open: bool,
    settings_page: settings::SettingsPage,
    settings_focus: FocusHandle,
    settings_restore: Option<FocusHandle>,
    settings_project: Option<settings_project::ProjectSettings>,
    settings_tab: settings::SettingsTab,
    /// Shared by both pages: opening either starts it at the top.
    settings_scroll: ScrollHandle,
    keyboard: settings_keyboard::KeyboardState,
    /// Settings → Editor: the custom command fields.
    editor_settings: settings_editor::EditorSettings,
    /// Open in Editor, Show in Finder, and Open Terminal Here.
    ways_out: ways_out::WaysOut,
    policy: settings_policy::PolicyState,
    cleanup: cleanup::CleanupState,
    agent: settings_agent::AgentState,
    harnesses: harnesses::HarnessesState,
    providers: settings_providers::ProviderState,
    /// A thread was created at once (a worktree thread): open it when the host
    /// confirms creation, unless the user has gone elsewhere in the meantime.
    new_thread_open: bool,
    /// The new thread being written, before anything exists on the host.
    draft: new_thread::Draft,
    /// A worktree is being created or removed.
    admission: admission::State,
    build_output: build_output::State,
    worktree_busy: bool,
    worktrees: worktrees_view::View,
    worktree_pending: Option<worktree::Pending>,
    approval_keys: live_conversation::ApprovalKeys,
    /// The command palette or model picker, above the workspace.
    overlay: Option<commands::Overlay>,
    panels: panels::Panels,
    focus: FocusHandle,
    /// The conversation field that gets focus back after changing views.
    conversation_focus: Option<FocusHandle>,
    /// The window control that had focus before conversation find opened.
    conversation_find_focus: Option<FocusHandle>,
    sidebar: bool,
    /// A left press on the window bar may become a window move.
    window_drag: bool,
    diagnostics: bool,
    preferences: jump::PreferenceState,
    themes: appearance::Themes,
    code_font: appearance::CodeFont,
    updates: update::UpdateState,
    session: session::SessionState,
    picker_open: bool,
    notice: Option<String>,
    last_render_us: u128,
    max_render_us: u128,
    startup: Instant,
    first_render_ms: Option<u128>,
    interaction: Option<Instant>,
    switch_us: u128,
}

impl Plait {
    pub fn new(
        path: PathBuf,
        startup: Instant,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let host = HostState::new(path.clone());
        let state = AppState::new(path);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let commit_message =
            cx.new(|cx| text_field::TextField::new("Commit message", "Commit message", cx));
        let mut this = Self {
            tabs: Default::default(),
            state,
            host,
            navigation: navigation::Navigation::new(cx),
            workspace: thread_workspace::ThreadWorkspace::new(commit_message),
            commit_sends: Default::default(),
            settings_open: false,
            settings_page: Default::default(),
            settings_focus: cx.focus_handle(),
            settings_restore: None,
            settings_project: None,
            settings_tab: Default::default(),
            settings_scroll: ScrollHandle::new(),
            keyboard: Default::default(),
            editor_settings: Default::default(),
            ways_out: Default::default(),
            policy: Default::default(),
            cleanup: Default::default(),
            agent: Default::default(),
            harnesses: Default::default(),
            providers: Default::default(),
            new_thread_open: false,
            draft: Default::default(),
            admission: Default::default(),
            build_output: Default::default(),
            worktree_busy: false,
            worktrees: Default::default(),
            worktree_pending: None,
            approval_keys: Default::default(),
            overlay: None,
            panels: Default::default(),
            focus,
            conversation_focus: None,
            conversation_find_focus: None,
            sidebar: true,
            window_drag: false,
            diagnostics: false,
            preferences: jump::PreferenceState::default(),
            themes: Default::default(),
            code_font: Default::default(),
            updates: update::UpdateState::default(),
            session: session::SessionState::default(),
            picker_open: false,
            notice: None,
            last_render_us: 0,
            max_render_us: 0,
            startup,
            first_render_ms: None,
            interaction: None,
            switch_us: 0,
        };
        this.workspace
            .live_scroll
            .set_scroll_handler(cx.listener(Self::scroll_history));
        this.initialize_navigation(window, cx);
        this.pump(cx);
        this.load_preferences(cx);
        this.observe_appearance(window, cx);
        this.apply_theme(cx);
        this.load_themes(cx);
        // An open picker shows host state; keep it current as replies land.
        cx.observe_self(|this, cx| {
            this.sync_picker(cx);
            this.sync_policy(cx);
            this.sync_cleanup(cx);
            this.sync_admission_capability(cx);
            this.sync_admission_context(cx);
            this.sync_agent(cx);
            this.sync_project_settings(cx);
            this.sync_harnesses(cx);
            this.sync_provider_settings(cx);
        })
        .detach();
        // Keys dispatch from the focused element. One that is no longer drawn,
        // such as a field on a tab that was left, sends them from outside the
        // app's key context, where no shortcut or menu action is handled.
        cx.on_focus_lost(window, |this, window, cx| {
            window.focus(
                if this.settings_open {
                    &this.settings_focus
                } else {
                    &this.focus
                },
                cx,
            )
        })
        .detach();
        this.keep_focus_in_settings(window, cx);
        this.watch_approval_keys(window, cx);
        this.reconnect_host(window, cx);
        // Test scenes stay offline.
        if !cfg!(test) {
            this.check_for_updates(true, cx);
        }
        this
    }

    fn pump(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.state.next_repository_job() else {
            return;
        };
        let task = cx.background_executor().spawn(async move {
            let reply = job.run();
            (job, reply)
        });
        cx.spawn(async move |this, cx| {
            let (job, reply) = task.await;
            let _ = this.update(cx, |this, cx| {
                let status = matches!(job.job, plait_core::state::Job::Status { .. });
                let write = matches!(job.job, plait_core::state::Job::Write { .. });
                let owner = job.owner;
                let active = this.state.complete_repository_job(job, reply);
                if write {
                    this.after_git_write(owner, cx);
                }
                if !active {
                    this.pump(cx);
                    return;
                }
                if write {
                    this.refresh_turn_review(cx);
                }
                this.sync_recorded_current(cx);
                if status {
                    this.thread_files_status(cx);
                    this.check_file(cx);
                }
                this.normalize_checkout(cx);
                this.bind_selected_checkout();
                this.deliver_files(cx);
                this.deliver_quick_open(cx);
                this.deliver_branches(cx);
                this.restore_ready_bookmarks(cx);
                this.pump(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn bind_selected_checkout(&mut self) -> bool {
        let Some(id) = self.host.thread.selected.clone() else {
            return true;
        };
        let Some(key) = self.state.checkout_key() else {
            return false;
        };
        match self.host.bind_checkout(&id, key) {
            Ok(()) => true,
            Err(error) => {
                self.notice = Some(error);
                false
            }
        }
    }

    /// The open checkout is known to differ from the selected thread's. An
    /// identity not read yet is not a mismatch: Git status may be slow or
    /// fail, and a send verifies the bound directory before it leaves.
    fn checkout_mismatch(&self) -> bool {
        let Some(id) = self.host.thread.selected.as_deref() else {
            return false;
        };
        match (self.state.checkout_key(), self.host.expected_checkout(id)) {
            (Some(key), Some(expected)) => key != expected,
            _ => false,
        }
    }

    fn checkout_matches_thread(&self) -> bool {
        let Some(id) = self.host.thread.selected.as_deref() else {
            return true;
        };
        self.state
            .checkout_key()
            .is_some_and(|key| self.host.expected_checkout(id) == Some(key))
    }

    /// Move to the next or previous host thread, wrapping. With no selection,
    /// forward starts at the first thread and backward at the last.
    fn step_thread(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let indices = self.filtered_threads(cx);
        let count = indices.len();
        if count == 0 {
            return;
        }
        let current = self.host.thread.selected.as_ref().and_then(|id| {
            indices
                .iter()
                .position(|&index| &self.host.connection.threads[index].id == id)
        });
        let next = match (current, forward) {
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        self.interaction = Some(std::time::Instant::now());
        let id = self.host.connection.threads[indices[next]].id.clone();
        self.select_host_thread(id, window, cx);
    }

    fn select_file(
        &mut self,
        file: usize,
        mode: DiffMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let same_recorded_file = self.workspace.turn_review.current
            && self.recorded_file().is_some_and(|recorded| {
                self.state
                    .repository
                    .snapshot()
                    .ready()
                    .and_then(|s| s.changes.get(file))
                    .is_some_and(|current| current.path == recorded.path)
            });
        if !same_recorded_file {
            self.clear_turn_review();
        }
        self.show_file(file, mode, None, window, cx);
    }

    /// `step` carries the direction when moving through the list by keyboard,
    /// which reads the following file ahead.
    fn show_file(
        &mut self,
        file: usize,
        mode: DiffMode,
        step: Option<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.restore_pending[1] = false;
        self.pause_conversation(Some(window), cx);
        window.focus(&self.focus, cx);
        self.interaction = Some(Instant::now());
        match step {
            Some(forward) => self.state.repository.step(file, mode, forward),
            None => self.state.repository.select(file, mode),
        }
        self.workspace.diff_scroll = UniformListScrollHandle::new();
        self.workspace.tab = Workspace::Changes;
        self.refresh_turn_review(cx);
        self.pump(cx);
        cx.notify();
    }

    fn step_file(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(record) = &self.workspace.turn_review.record {
            let count = record.files.len();
            if count > 0 {
                let selected = self.workspace.turn_review.selected.unwrap_or(0);
                let next = if forward {
                    (selected + 1) % count
                } else {
                    (selected + count - 1) % count
                };
                self.workspace
                    .file_scroll
                    .scroll_to_item(next, ScrollStrategy::Top);
                self.select_recorded_file(next, window, cx);
            }
            return;
        }
        let repo = &self.state.repository;
        let Some(snapshot) = repo.snapshot().ready() else {
            return;
        };
        let count = snapshot.changes.len();
        if count == 0 {
            return;
        }
        let current = repo.selected_diff().as_ref().map_or(0, |s| s.file);
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        let mode = if snapshot.changes[next].unstaged() {
            DiffMode::Unstaged
        } else {
            DiffMode::Staged
        };
        self.workspace
            .file_scroll
            .scroll_to_item(next, ScrollStrategy::Top);
        self.show_file(next, mode, Some(forward), window, cx);
    }

    fn refresh(&mut self, _: &Refresh, window: &mut Window, cx: &mut Context<Self>) {
        if self.worktrees_visible() {
            self.refresh_worktrees_view(true, cx);
            return;
        }
        window.focus(&self.focus, cx);
        self.fetch_turn_records(window, cx);
        self.refresh_turn_review(cx);
        if !matches!(self.state.repository.snapshot(), Load::Loading) {
            self.state.repository.refresh();
            self.pump(cx);
            cx.notify();
        }
    }

    fn open_repository(&mut self, _: &OpenRepository, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        if self.picker_open {
            return;
        }
        if self.host.creation.pending_create.is_some() || self.host.creation.creating {
            self.notice =
                Some("Resolve the pending thread creation before changing checkout.".into());
            cx.notify();
            return;
        }
        self.picker_open = true;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            // Opening only reads the folder; threads and Git writes come later, by choice.
            prompt: Some("Open repository".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = prompt.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.picker_open = false;
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            this.change_checkout(path, window, cx);
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => {
                        this.notice = Some(format!("Cannot open folder picker: {error}"))
                    }
                    Err(error) => {
                        this.notice = Some(format!("Folder picker closed unexpectedly: {error}"))
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Plait {
    /// Open the application's Settings, or close whichever settings page is open.
    fn show_settings(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open_page(
            open.then_some(settings::SettingsPage::Application),
            window,
            cx,
        );
    }

    /// Open the active project's settings page.
    pub(super) fn show_project_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_settings_for_project(self.visible_project_checkout().to_owned(), window, cx);
    }

    /// Cmd-comma: the application page closes when it is the one open;
    /// from anywhere else, including the project page, it opens.
    fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = self.settings_open && self.settings_page == settings::SettingsPage::Application;
        self.show_settings(!open, window, cx);
    }

    /// Show `page` over the workspace, or close settings with `None`.
    fn open_page(
        &mut self,
        page: Option<settings::SettingsPage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_overlay(true, window, cx);
        if page.is_some() && !self.settings_open {
            self.settings_restore = window.focused(cx);
        }
        self.settings_open = page.is_some();
        self.dismiss_tab_controls();
        self.cover_workspace(window, cx);
        if self.settings_open {
            window.focus(&self.settings_focus, cx);
        } else {
            let restore = self
                .settings_restore
                .take()
                .filter(|focus| focus.is_rendered(window));
            window.focus(restore.as_ref().unwrap_or(&self.focus), cx);
        }
        self.settings_page = page.unwrap_or_default();
        self.settings_scroll.set_offset(point(px(0.), px(0.)));
        self.cancel_recording(cx);
        match page {
            Some(settings::SettingsPage::Application) => {
                self.prepare_keyboard(cx);
                self.prepare_code_font(cx);
                self.prepare_editor_settings(cx);
                self.prepare_providers(cx);
            }
            Some(settings::SettingsPage::Project) => {
                self.close_provider_form(cx);
                // Opened fresh, the page asks before trusting again.
                self.project_policy_mut().confirm = false;
                // Pi's files can change while the app is open.
                self.refresh_project_agent(cx);
            }
            None => self.close_provider_form(cx),
        }
        if page.is_none()
            && self.workspace.tab == Workspace::Conversation
            && let Some(id) = self.host.thread.selected.clone()
        {
            self.mark_thread_read(&id, cx);
        }
        cx.notify();
    }

    fn wide(window: &Window) -> bool {
        window.viewport_size().width > px(WIDE_WINDOW)
    }

    /// The Git inspector beside the conversation: wide windows, unless hidden.
    pub(super) fn shows_inspector(&self, wide: bool) -> bool {
        wide && !self.worktrees_visible()
            && self.tabs.model.active().is_some()
            && !self.preferences.value.hide_inspector
    }

    /// Show or hide the Git inspector and remember the choice. A narrow window
    /// has no room for it, so the toggle says where Git is instead.
    fn toggle_inspector(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !Self::wide(window) {
            self.workspace.inspector_note = !self.workspace.inspector_note;
            cx.notify();
            return;
        }
        self.workspace.inspector_note = false;
        let mut value = self.preferences.value.clone();
        value.hide_inspector = !value.hide_inspector;
        self.set_preferences(value, jump::PreferenceGroup::View, cx);
    }

    fn inspector_note(&self, cx: &mut Context<Self>) -> impl IntoElement {
        row()
            .id("inspector-note")
            .role(Role::Group)
            .aria_label("Git inspector unavailable at this window width")
            .audited()
            .flex_shrink_0()
            .px_4()
            .py_2()
            .gap_3()
            .border_b_1()
            .border_color(rgb(colors().border))
            .text_size(px(12.))
            .child(div().flex_1().min_w_0().text_color(rgb(colors().muted)).child(a11y_text(
                "inspector-note-detail",
                format!(
                    "The Git inspector needs a window wider than {WIDE_WINDOW} points. Git status and diffs are in Changes."
                ),
            )))
            .child(
                toned_button("inspector-note-changes", "Show Changes", Tone::Filled).on_activate(
                    cx,
                    |this, window, cx| {
                        this.workspace.inspector_note = false;
                        this.show_tab(Workspace::Changes, window, cx);
                    },
                ),
            )
            .child(
                button("inspector-note-dismiss", "Dismiss").on_activate(cx, |this, _, cx| {
                    this.workspace.inspector_note = false;
                    cx.notify();
                }),
            )
    }

    fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Keep the shortcut context alive when its focused child is removed.
        window.focus(&self.focus, cx);
        self.sidebar = !self.sidebar;
        cx.notify();
    }

    fn show_tab(&mut self, tab: Workspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.worktrees_visible() {
            return;
        }
        if !self.ensure_view_tab() {
            cx.notify();
            return;
        }
        if tab == Workspace::Conversation && self.draft.open && self.draft.composer.is_none() {
            self.open_draft_content(window, cx);
        }
        self.tabs.lifecycle.view_revision += 1;
        let conversation = Workspace::Conversation;
        let previous = self.workspace.tab;
        if self.workspace.tab == conversation && tab != conversation {
            self.pause_conversation(Some(window), cx);
            self.leave_conversation_focus(window, cx);
        } else if self.workspace.tab != conversation && tab == conversation {
            self.return_conversation_focus(window, cx);
        }
        self.workspace.tab = tab;
        if tab == conversation
            && previous != conversation
            && self.host.connection.connected
            && !cfg!(test)
            && let Some(id) = self.host.thread.selected.clone()
        {
            self.watch_thread(id, window, cx);
        }
        self.queue_tab_save(cx);
        if tab == Workspace::Changes {
            if self.workspace.restore_pending[1]
                && let Some(saved) = self.workspace.restore.as_ref()
                && let Some(path) = saved.changes.path.as_ref()
            {
                self.state.repository.restore_path(
                    path.into(),
                    if saved.changes.staged {
                        DiffMode::Staged
                    } else {
                        DiffMode::Unstaged
                    },
                );
                self.pump(cx);
            }
            self.refresh_turn_review(cx);
        }
        if tab == Workspace::History {
            self.state.repository.request_history();
            if self.workspace.restore_pending[2]
                && let Some(commit) = self
                    .workspace
                    .restore
                    .as_ref()
                    .and_then(|b| b.history.commit.clone())
            {
                self.state.repository.select_commit(commit);
            }
            self.pump(cx);
        } else {
            self.state.repository.leave_history();
        }
        if tab == Workspace::Files {
            self.check_file(cx);
        }
        if tab == Workspace::Conversation
            && self.conversation_snapshot().is_some()
            && window.is_window_active()
            && !self.settings_open
            && let Some(id) = self.host.thread.selected.clone()
        {
            self.mark_thread_read(&id, cx);
        }
        cx.notify();
    }

    /// Where the conversation can hold the keyboard: its composer, and its
    /// find field while the bar is open.
    fn conversation_fields(&self, cx: &App) -> [Option<FocusHandle>; 2] {
        let find = &self.workspace.conversation_find;
        [
            self.visible_composer()
                .map(|composer| composer.focus_handle(cx)),
            find.field
                .as_ref()
                .filter(|_| find.open)
                .map(|field| field.focus_handle(cx)),
        ]
    }

    /// Another tab hides the conversation: the keyboard leaves its fields for
    /// the root, which every tab draws, and they remember it for the return.
    fn leave_conversation_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.conversation_focus = self
            .conversation_fields(cx)
            .into_iter()
            .flatten()
            .find(|field| field.contains_focused(window, cx));
        if self.conversation_focus.is_some() {
            window.focus(&self.focus, cx);
        }
    }

    /// The conversation is shown again: give the keyboard back to the field
    /// that had it, if it is still there.
    fn return_conversation_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(field) = self.conversation_focus.take() else {
            return;
        };
        if !self.settings_open && self.conversation_fields(cx).contains(&Some(field.clone())) {
            window.focus(&field, cx);
        }
    }

    /// The active content supplies the second row; an empty workspace has none.
    fn header(
        &self,
        _title: String,
        _location: Option<String>,
        _status: (String, u32),
        wide: bool,
        _fullscreen: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.tabs.model.active().is_none() || self.worktrees_visible() {
            return div().into_any_element();
        }
        let header = row()
            .h(px(36.))
            .flex_shrink_0()
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(rgb(colors().border));
        let project = self.active_project();
        let branch = self
            .state
            .repository
            .snapshot()
            .ready()
            .map(|s| s.branch.as_str());
        let full = format!(
            "{} / {}{}\n{}",
            project.name,
            project.machines[0].name,
            branch.map(|b| format!(" · {b}")).unwrap_or_default(),
            self.host.creation.checkout.display()
        );
        let context = format!(
            "{}{}{}",
            project.name,
            if wide {
                format!(" / {}", project.machines[0].name)
            } else {
                String::new()
            },
            branch.map(|b| format!(" · {b}")).unwrap_or_default()
        );
        header
            .child(
                div()
                    .id("workspace-context")
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(rgb(colors().muted))
                    .child(context)
                    .tooltip(move |_, cx| {
                        cx.new(|_| thread_controls::TextTooltip(full.clone()))
                            .into()
                    }),
            )
            .children(self.cleanup_control(cx))
            .child(self.workspace_switch(cx))
            .child(
                icon_button(
                    "toggle-inspector",
                    "panel-right",
                    if wide {
                        "Toggle Git inspector"
                    } else {
                        "Git inspector (needs a wider window)"
                    },
                )
                .map(|el| self.covered_tab_control(el))
                .aria_keyshortcuts(self.shortcut_aria("view.toggle_inspector"))
                .on_activate(cx, |this, window, cx| this.toggle_inspector(window, cx)),
            )
            .into_any_element()
    }

    /// Conversation, Changes, History, and Files: the views of the open checkout.
    fn workspace_switch(&self, cx: &mut Context<Self>) -> Div {
        let views = self
            .tabs
            .model
            .active()
            .and_then(|id| self.tabs.model.get(id))
            .map(|tab| tab.content().view_commands())
            .unwrap_or_default();
        let changed = self
            .state
            .repository
            .snapshot()
            .ready()
            .map(|s| s.changes.len())
            .filter(|n| *n > 0);
        let mut controls = segmented();
        for &view in views {
            let (id, title, action, hover) = match view {
                Workspace::Conversation => (
                    "conversation-tab",
                    "Conversation",
                    "view.conversation",
                    "Read this thread and write a message.",
                ),
                Workspace::Changes => (
                    "changes-tab",
                    "Changes",
                    "view.changes",
                    "Review changes in this thread’s checkout.",
                ),
                Workspace::History => (
                    "history-tab",
                    "History",
                    "view.history",
                    "Read commits in this thread’s checkout.",
                ),
                Workspace::Files => (
                    "files-tab",
                    "Files",
                    "view.files",
                    "Read files in this thread’s checkout.",
                ),
            };
            controls = controls.child(
                segment(id, title, self.workspace.tab == view)
                    .map(|el| self.covered_tab_control(el))
                    .aria_keyshortcuts(self.shortcut_aria(action))
                    .tooltip(move |_, cx| {
                        cx.new(|_| thread_controls::TextTooltip(hover.into()))
                            .into()
                    })
                    .when_some(changed.filter(|_| view == Workspace::Changes), |el, n| {
                        el.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(colors().subtle))
                                .child(n.to_string()),
                        )
                    })
                    .on_activate(cx, move |this, window, cx| this.show_tab(view, window, cx)),
            );
        }
        controls
    }

    /// Where the selected thread runs: Project, Machine, and its checkout branch.
    pub(super) fn environment(
        &self,
        model: Option<String>,
        note: Option<(&'static str, u32)>,
    ) -> impl IntoElement {
        // Host threads belong to this machine's checkout.
        let project = self.active_project();
        let machine = &project.machines[0];
        let branch = self
            .state
            .repository
            .snapshot()
            .ready()
            .map(|snapshot| snapshot.branch.clone());
        let chip = |id: &'static str, glyph: &'static str, label: String| {
            row()
                .gap(px(5.))
                .min_w_0()
                .child(icon(glyph).size(px(12.)).text_color(rgb(colors().subtle)))
                .child(div().truncate().child(a11y_text(id, label)))
        };
        row()
            .w_full()
            .max_w(px(CONTENT_WIDTH))
            .mx_auto()
            .px_1()
            .gap_4()
            .text_size(px(11.))
            .text_color(rgb(colors().muted))
            .child(chip("environment-project", "folder", project.name.clone()))
            .child(chip("environment-machine", "machine", machine.name.clone()))
            .when_some(branch, |el, branch| {
                el.child(chip("environment-branch", "branch", branch))
            })
            .when_some(model, |el, model| {
                el.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(rgb(colors().subtle))
                        .child(a11y_text("environment-model", model)),
                )
            })
            .child(div().flex_1())
            .when_some(note, |el, (note, tint)| {
                el.child(
                    tag("environment-note", note, tint)
                        .flex_shrink_0()
                        .whitespace_nowrap(),
                )
            })
    }

    fn diagnostics_bar(&self) -> impl IntoElement {
        row()
            .h(px(24.))
            .px_3()
            .flex_shrink_0()
            .justify_end()
            .bg(rgb(colors().rail))
            .border_t_1()
            .border_color(rgb(colors().border))
            .text_size(px(11.))
            .text_color(rgb(colors().subtle))
            .child(a11y_text(
                "diagnostics-summary",
                format!(
                    "UI {} µs · peak {} µs · switch {} µs · first {} ms",
                    self.last_render_us,
                    self.max_render_us,
                    self.switch_us,
                    self.first_render_ms.unwrap_or(0)
                ),
            ))
    }
}

impl Render for Plait {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let start = Instant::now();
        self.first_render_ms
            .get_or_insert_with(|| self.startup.elapsed().as_millis());
        if let Some(interaction) = self.interaction.take() {
            self.switch_us = interaction.elapsed().as_micros();
        }
        let wide = Self::wide(window);
        self.cover_workspace(window, cx);
        // Full screen hides the traffic lights the header otherwise clears.
        let fullscreen = window.is_fullscreen();
        self.panels.viewport = f32::from(window.viewport_size().width);
        self.advance_reveals(window);
        self.settle_find_reveal(window);
        let project = self.active_project();
        let machine = &project.machines[0];
        let real = self.host.thread.selected.is_some();
        let connection = if self.host.connection.connecting {
            ("Connecting…".to_owned(), colors().muted)
        } else if self.host.connection.connected {
            ("Host connected".to_owned(), colors().mint)
        } else {
            ("Host disconnected".to_owned(), colors().amber)
        };
        let drafting = !real && self.draft.open;
        let title = if real {
            self.thread_title(self.host.thread.selected.as_deref().unwrap_or_default())
        } else if drafting {
            session::UNTITLED.to_owned()
        } else {
            project.name.clone()
        };
        let status = if real {
            self.host.thread.snapshot.as_ref().map_or_else(
                || ("Loading".to_owned(), colors().muted),
                |snapshot| {
                    (
                        snapshot.status.label().to_owned(),
                        host_status_color(snapshot.status),
                    )
                },
            )
        } else {
            connection
        };
        // With no thread open the title already names the project.
        let location = if real || drafting {
            Some(format!("{} / {}", project.name, machine.name))
        } else {
            Some(machine.name.clone())
        };
        let root = column()
            .id("plait")
            .role(Role::Application)
            .aria_label("Plait")
            .key_context("Plait")
            .track_focus(&self.focus)
            .map(|root| self.settings_shortcuts(root, cx))
            .on_action(cx.listener(Self::open_repository))
            .on_action(cx.listener(Self::refresh))
            .on_action(
                cx.listener(|this, _: &worktrees_view::Worktrees, window, cx| {
                    this.open_worktrees(this.visible_project_checkout().to_owned(), window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &Settings, window, cx| this.toggle_settings(window, cx)),
            )
            .on_action(cx.listener(|this, _: &ProjectSettings, window, cx| {
                this.show_project_settings(window, cx)
            }))
            .on_action(cx.listener(|this, _: &CheckForUpdates, window, cx| {
                this.settings_tab = settings::SettingsTab::About;
                this.show_settings(true, window, cx);
                this.check_for_updates(false, cx);
            }))
            .on_action(
                cx.listener(|this, _: &ToggleSidebar, window, cx| this.toggle_sidebar(window, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleInspector, window, cx| {
                this.toggle_inspector(window, cx)
            }))
            .on_action(cx.listener(|this, _: &RenameThread, window, cx| {
                if let Some(id) = this.host.thread.selected.clone() {
                    this.begin_rename(id, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ArchiveThread, window, cx| {
                if let Some(thread) = this
                    .host
                    .connection
                    .threads
                    .iter()
                    .find(|thread| Some(&thread.id) == this.host.thread.selected.as_ref())
                {
                    this.archive_thread(
                        thread.id.clone(),
                        thread.archived != Some(true),
                        window,
                        cx,
                    );
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleArchived, _, cx| {
                this.navigation.archived = !this.navigation.archived;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FilterThreads, window, cx| {
                this.sidebar = true;
                window.focus(&this.navigation.filter.focus_handle(cx), cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleNotifications, _, cx| {
                let mut value = this.preferences.value.clone();
                value.navigation.mute_notifications = !value.navigation.mute_notifications;
                this.set_preferences(value, jump::PreferenceGroup::View, cx);
            }))
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| {
                this.toggle_command_palette(window, cx)
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && this.workspace.images.enlarged.take().is_some()
                {
                    this.focus_composer(window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &NewThread, window, cx| this.open_draft(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleChangedFiles, _, cx| {
                let hidden = !this.preferences.value.hide_changed_files;
                this.set_changed_files_hidden(hidden, cx)
            }))
            .on_action(cx.listener(|this, _: &NewWorktreeThread, window, cx| {
                this.new_worktree_thread(window, cx)
            }))
            .on_action(cx.listener(|this, _: &NewThreadWithModel, window, cx| {
                this.open_draft(window, cx);
                this.open_model_picker(model_picker::ModelPurpose::NewThread, window, cx)
            }))
            .on_action(cx.listener(|this, _: &SwitchThreadModel, window, cx| {
                this.open_thread_models(window, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleDiagnostics, _, cx| {
                this.diagnostics = !this.diagnostics;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Conversation, window, cx| {
                this.show_tab(Workspace::Conversation, window, cx)
            }))
            .on_action(cx.listener(|this, _: &Changes, window, cx| {
                this.show_tab(Workspace::Changes, window, cx)
            }))
            .on_action(cx.listener(|this, _: &CommitHistory, window, cx| {
                this.show_tab(Workspace::History, window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowFiles, window, cx| {
                this.show_tab(Workspace::Files, window, cx)
            }))
            .on_action(cx.listener(|this, _: &QuickOpen, window, cx| this.quick_open(window, cx)))
            .on_action(cx.listener(|this, _: &FindInFile, window, cx| {
                if let Some(target) = this.find_target() {
                    this.open_find_in(target, window, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &FindNext, _, cx| {
                if let Some(target) = this.find_target() {
                    this.step_find_in(target, true, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &FindPrevious, _, cx| {
                if let Some(target) = this.find_target() {
                    this.step_find_in(target, false, cx)
                }
            }))
            // Show Changes, where the result and any error appear.
            .on_action(cx.listener(|this, _: &StageAll, window, cx| {
                this.show_tab(Workspace::Changes, window, cx);
                this.git_write(plait_core::git::WriteOp::StageAll, cx)
            }))
            .on_action(cx.listener(|this, _: &UnstageAll, window, cx| {
                this.show_tab(Workspace::Changes, window, cx);
                this.git_write(plait_core::git::WriteOp::UnstageAll, cx)
            }))
            .on_action(
                cx.listener(|this, _: &CommitStaged, window, cx| this.focus_commit(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Push, window, cx| {
                this.show_tab(Workspace::Changes, window, cx);
                this.push(cx)
            }))
            .on_action(cx.listener(|this, _: &Pull, window, cx| {
                this.show_tab(Workspace::Changes, window, cx);
                this.pull(cx)
            }))
            .on_action(cx.listener(|this, _: &Fetch, window, cx| {
                this.show_tab(Workspace::Changes, window, cx);
                this.git_write(plait_core::git::WriteOp::Fetch, cx)
            }))
            .on_action(cx.listener(|this, _: &SwitchBranch, window, cx| {
                this.open_branches(false, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &NewBranch, window, cx| this.open_branches(true, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &NextThread, window, cx| this.step_thread(true, window, cx)),
            )
            .on_action(cx.listener(|this, _: &PreviousThread, window, cx| {
                this.step_thread(false, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &NextFile, window, cx| this.step_file(true, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &PreviousFile, window, cx| this.step_file(false, window, cx)),
            )
            .on_action(cx.listener(|_, _: &Tab, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &BackTab, window, cx| window.focus_prev(cx)))
            .on_action(
                cx.listener(|this, _: &Close, window, cx| this.close_desktop(false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Quit, window, cx| this.close_desktop(true, window, cx)),
            )
            .on_action(cx.listener(|this, _: &ProviderSettings, window, cx| {
                this.show_settings(true, window, cx);
                this.settings_tab = settings::SettingsTab::Providers;
            }))
            .on_action(cx.listener(|_, _: &Minimize, window, _| window.minimize_window()))
            .map(|root| self.on_jump_actions(root, cx))
            .map(|root| self.on_tab_actions(root, cx))
            .capture_key_up(cx.listener(|this, _, _, cx| this.queue_tab_save(cx)))
            .capture_any_mouse_up(cx.listener(|this, _, _, cx| this.queue_tab_save(cx)))
            .map(|root| self.on_ways_out_actions(root, cx))
            .on_action(cx.listener(|this, action: &UndoLastTurn, window, cx| {
                if !this.defer_key_action(action, cx) {
                    this.undo_last_turn(window, cx);
                }
            }))
            .map(|root| self.on_approval_actions(root, cx))
            .map(|root| self.on_selection_actions(root, cx))
            .map(|root| self.on_keyboard_controls(root, cx))
            .map(|root| self.on_panel_resize(root, cx))
            .size_full()
            .overflow_hidden()
            .bg(rgb(colors().bg))
            .text_color(rgb(colors().text))
            .text_size(px(13.))
            .font_family(".SystemUIFont")
            .child(self.tab_save_input(cx))
            .child(self.tab_row(window, cx))
            .child(
                row()
                    .gap_0()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .when(self.sidebar, |el| el.child(self.navigation(cx)))
                    .child(
                        column()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(self.header(title, location, status, wide, fullscreen, cx))
                            .when_some(self.session.note.clone(), |el, note| {
                                el.child(
                                    div()
                                        .px_4()
                                        .py_2()
                                        .text_size(px(12.))
                                        .text_color(rgb(colors().amber))
                                        .child(note),
                                )
                            })
                            .when_some(self.notice.clone(), |el, notice| {
                                el.child(
                                    div()
                                        .px_4()
                                        .py_2()
                                        .text_size(px(12.))
                                        .text_color(rgb(colors().red))
                                        .child(notice),
                                )
                            })
                            .when_some(self.build_progress_bar(cx), |el, bar| el.child(bar))
                            .when_some(self.creation_bar(cx), |el, bar| el.child(bar))
                            .when(self.workspace.inspector_note && !wide, |el| {
                                el.child(self.inspector_note(cx))
                            })
                            .child(if self.worktrees_visible() {
                                self.worktrees_view(wide, cx)
                            } else if self.workspace.tab == Workspace::History {
                                self.history_view(wide, cx).into_any_element()
                            } else if self.workspace.tab == Workspace::Files {
                                self.file_view(cx)
                            } else if self.workspace.tab == Workspace::Changes {
                                self.changes(wide, cx).into_any_element()
                            } else if real {
                                self.live_conversation(wide, cx).into_any_element()
                            } else if drafting {
                                self.draft_view(wide, cx).into_any_element()
                            } else {
                                self.idle(wide, cx).into_any_element()
                            }),
                    ),
            )
            // Timings stay available without spending a permanent chrome row on them.
            .when(self.diagnostics, |el| el.child(self.diagnostics_bar()))
            .when_some(self.tab_list(cx), |el, list| el.child(list))
            .when_some(self.image_overlay(window, cx), |el, overlay| {
                el.child(overlay)
            })
            .when(self.settings_open, |el| {
                el.child(self.settings_modal(window, cx))
            })
            // Pickers remain above Settings.
            .when_some(self.overlay.as_ref(), |el, overlay| {
                el.child(overlay.palette.clone())
            });

        self.last_render_us = start.elapsed().as_micros();
        self.max_render_us = self.max_render_us.max(self.last_render_us);
        root.audited()
    }
}

/// Bind the default shortcuts and build the menu bar from the action table.
pub fn menus(cx: &mut App) {
    actions::install(cx, &actions::KeyOverrides::new());
    cx.on_action(|_: &Quit, cx| cx.quit());
}
