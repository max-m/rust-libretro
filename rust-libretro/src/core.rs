use crate::*;

/// This trait defines the basic functions that every libretro core must implement.
/// See also [`retro_core!()`].
pub trait Core {
    /// Returns static info about this core.
    fn get_info(&self) -> SystemInfo;

    /// Called when the frontend needs information about the
    /// audio and video timings and the video geometry.
    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info;

    /// Called when the frontend set a new environment callback.
    ///
    /// Guaranteed to be called before [`Core::on_init`].
    fn on_set_environment(&mut self, _initial: bool, _ctx: &mut SetEnvironmentContext) {}

    /// Called when the libretro API has been initialized.
    fn on_init(&mut self, _ctx: &mut InitContext) {
        todo!("on_init");
    }

    /// Called when the libretro API gets destucted.
    fn on_deinit(&mut self, _ctx: &mut DeinitContext) {}

    /// TODO
    fn on_set_controller_port_device(
        &mut self,
        _port: std::os::raw::c_uint,
        _device: std::os::raw::c_uint,
    ) {
        todo!("on_set_controller_port_device");
    }

    /// Called when the frontend requests resetting the system.
    fn on_reset(&mut self, _ctx: &mut ResetContext) {}

    /// Called once per frame
    ///
    /// If a frame is not rendered for reasons where a game "dropped" a frame,
    /// this still counts as a frame, and [`Core::on_run`] should explicitly dupe
    /// a frame if [`environment::can_dupe`] returns [`true`].
    /// In this case, the video callback can take a NULL argument for data.
    fn on_run(&mut self, _ctx: &mut RunContext, _delta_us: Option<i64>) {}

    /// Returns the amount of data the implementation requires to serialize
    /// internal state (save states).
    /// Between calls to [`Core::on_load_game`] and [`Core::on_unload_game`], the
    /// returned size is never allowed to be larger than a previous returned
    /// value, to ensure that the frontend can allocate a save state buffer once.
    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> size_t {
        0
    }

    /// Serializes internal state. If failed, or size is lower than
    /// [`Core::get_serialize_size`], it should return [`false`], [`true'] otherwise.
    /// TODO
    fn on_serialize(
        &mut self,
        _data: *mut std::os::raw::c_void,
        _size: size_t,
        _ctx: &mut SerializeContext,
    ) -> bool {
        false
    }

    /// Deserializes internal state.
    /// TODO
    fn on_unserialize(
        &mut self,
        _data: *const std::os::raw::c_void,
        _size: size_t,
        _ctx: &mut UnserializeContext,
    ) -> bool {
        false
    }

    /// Called when a game should be loaded.
    /// Return [`true`] to indicate successful loading and [`false`] to indicate load failure.
    fn on_load_game(&mut self, _game: Option<retro_game_info>, _ctx: &mut LoadGameContext) -> bool {
        false
    }

    /// Loads a "special" kind of game. Should not be used, except in extreme cases.
    /// TODO
    fn on_load_game_special(
        &mut self,
        _game_type: std::os::raw::c_uint,
        _info: *const retro_game_info,
        _num_info: size_t,
        _ctx: &mut LoadGameSpecialContext,
    ) -> bool {
        false
    }

    /// Called when the currently loaded game should be unloaded.
    /// Called before [`Core::on_deinit`].
    fn on_unload_game(&mut self, _ctx: &mut UnloadGameContext) {}

    /// TODO
    fn on_cheat_reset(&mut self, _ctx: &mut CheatResetContext) {}

    /// TODO
    fn on_cheat_set(
        &mut self,
        _index: std::os::raw::c_uint,
        _enabled: bool,
        _code: *const std::os::raw::c_char,
        _ctx: &mut CheatSetContext,
    ) {
    }

    /// Gets the region of the game.
    /// TODO
    fn on_get_region(&mut self, _ctx: &mut GetRegionContext) -> std::os::raw::c_uint {
        todo!("on_get_region");
    }

    /// TODO
    fn get_memory_data(
        &mut self,
        _id: std::os::raw::c_uint,
        _ctx: &mut GetMemoryDataContext,
    ) -> *mut std::os::raw::c_void {
        std::ptr::null_mut()
    }

    /// TODO
    fn get_memory_size(
        &mut self,
        _id: std::os::raw::c_uint,
        _ctx: &mut GetMemorySizeContext,
    ) -> size_t {
        0
    }

    /// Gets called when the core options have been changed.
    ///
    /// Options get checked before [`Core::on_load_game`], [`Core::on_load_game_special`] and before each call of [`Core::on_run`].
    fn on_options_changed(&mut self, _ctx: &mut OptionsChangedContext) {}

    fn on_keyboard_event(
        &mut self,
        _down: bool,
        _keycode: retro_key,
        _character: u32,
        _key_modifiers: retro_mod,
    ) {
    }

    /// Needed for the frame time callback and calculated as `1000000 / FPS`.
    /// Gets called right after [`Core::on_load_game`] or [`Core::on_load_game_special`].
    fn get_frame_time_reference(&self) -> retro_usec_t {
        0
    }

    /// Called when the frontend needs more audio frames
    fn on_write_audio(&mut self, _ctx: &mut AudioContext) {}
}
