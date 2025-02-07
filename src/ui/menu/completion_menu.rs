use mlua::{prelude::LuaResult, Lua};
use neovim::Api;

use crate::completion::CompletionItem;
use crate::settings::ui::border::Border;
use crate::ui::WindowPosition;

#[derive(Debug)]
pub struct CompletionMenu {
    /// The handle of the buffer used to show the completion items. It is set
    /// once on initialization and never changes.
    bufnr: u32,

    /// A namespace id used to handle the highlighting of characters matching
    /// the current completion prefix. It is set once on initialization and
    /// never changed.
    mc_nsid: u32,

    /// The index of the currently selected completion item, or `None` if no
    /// completion is selected.
    pub selected_index: Option<usize>,

    /// The width of the completion menu if it's currently visible, or `None`
    /// otherwise. Used by the details window to figure out where to position
    /// itself.
    pub width: Option<u32>,

    /// The handle of the floating window used to show the completion items,
    /// or `None` if the completion menu is not currently visible.
    pub winid: Option<u32>,
}

impl CompletionMenu {
    pub fn new(api: &Api) -> LuaResult<Self> {
        Ok(CompletionMenu {
            bufnr: api.create_buf(false, true)?,
            mc_nsid: api.create_namespace("compleet_matched_chars")?,
            selected_index: None,
            width: None,
            winid: None,
        })
    }
}

impl CompletionMenu {
    /// Closes the completion menu, while also resetting the selected
    /// completion and the window position to `None`.
    pub fn close(&mut self, api: &Api) -> LuaResult<()> {
        if let Some(winid) = self.winid {
            // For some reason it's necessary to reset the cursor before
            // closing the floating window, or the next window will have the
            // cursor at whatever row it was on in this window. This is kinda
            // weird as the cursor position should be a window-scoped option,
            // and the next time the menu is opened it will be in a completely
            // new window. The only thing they will share is the buffer, so
            // maybe that's why we need this.
            api.win_set_cursor(winid, 1, 0)?;

            api.win_hide(winid)?;
            self.winid = None;
        }
        self.selected_index = None;
        self.width = None;
        Ok(())
    }

    /// Fills the completion buffer with the completion results.
    pub fn fill(
        &mut self,
        lua: &Lua,
        api: &Api,
        completions: &[CompletionItem],
    ) -> LuaResult<()> {
        let lines = completions
            .iter()
            .map(|c| c.format.as_ref())
            .collect::<Vec<&str>>();

        api.buf_set_lines(self.bufnr, 0, -1, false, &lines)?;

        // Highlight the matching characters of every completion item.
        let mut id = 0u16;
        let opts = lua.create_table_with_capacity(0, 4)?;
        for (row, completion) in completions.iter().enumerate() {
            for (range, hl_group) in &completion.hl_ranges {
                id += 1;
                opts.set("id", id)?;
                opts.set("end_row", row)?;
                opts.set("end_col", range.end)?;
                opts.set("hl_group", *hl_group)?;
                opts.set("priority", 10000)?;
                api.buf_set_extmark(
                    self.bufnr,
                    self.mc_nsid,
                    row as u32,
                    range.start as u32,
                    opts.clone(),
                )?;
            }
        }

        Ok(())
    }

    /// Whether a completion item is currently selected.
    pub fn is_item_selected(&self) -> bool { self.selected_index.is_some() }

    /// Whether the completion menu is visible.
    pub fn is_visible(&self) -> bool { self.winid.is_some() }

    /// Moves the completion menu to a new position.
    pub fn shift(
        &mut self,
        lua: &Lua,
        api: &Api,
        position: &WindowPosition,
    ) -> LuaResult<()> {
        let winid = self
            .winid
            .expect("The completion menu is visible so it has a window id.");

        let opts = lua.create_table_with_capacity(0, 5)?;
        opts.set("relative", "cursor")?;
        opts.set("height", position.height)?;
        opts.set("width", position.width)?;
        opts.set("row", position.row)?;
        opts.set("col", position.col)?;

        api.win_set_config(winid, opts)?;

        self.width = Some(position.width);

        Ok(())
    }

    /// Spawns the completion menu at a specified position.
    pub fn spawn(
        &mut self,
        lua: &Lua,
        api: &Api,
        position: &WindowPosition,
        border: &Border,
    ) -> LuaResult<()> {
        let opts = lua.create_table_with_capacity(0, 9)?;
        opts.set("relative", "cursor")?;
        opts.set("height", position.height)?;
        opts.set("width", position.width)?;
        opts.set("row", position.row)?;
        opts.set("col", position.col)?;
        opts.set("focusable", false)?;
        opts.set("style", "minimal")?;
        opts.set("noautocmd", true)?;

        if border.enable {
            opts.set("border", border.style.to_lua(lua)?)?;
        }

        let winid = api.open_win(self.bufnr, false, opts)?;
        api.win_set_option(
            winid,
            "winhl",
            "CursorLine:CompleetMenuSelected,FloatBorder:CompleetMenuBorder,\
             Normal:CompleetMenu,Search:None",
        )?;
        api.win_set_option(winid, "scrolloff", 0)?;

        self.width = Some(position.width);
        self.winid = Some(winid);

        Ok(())
    }

    /// Selects a new completion.
    pub fn select(
        &mut self,
        api: &Api,
        new_selected_index: Option<usize>,
    ) -> LuaResult<()> {
        let winid = self
            .winid
            .expect("The completion menu is visible so it has a window id");

        match new_selected_index {
            Some(index) => {
                api.win_set_cursor(winid, (index + 1).try_into().unwrap(), 0)?;
                if self.selected_index.is_none() {
                    api.win_set_option(winid, "cursorline", true)?;
                }
            },

            None => api.win_set_option(winid, "cursorline", false)?,
        }

        self.selected_index = new_selected_index;

        Ok(())
    }
}
