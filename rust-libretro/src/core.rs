//! Provides the [`Core`] and [`CoreOptions`] traits.
use crate::*;

/// This trait defines the [`set_core_options`](CoreOptions::set_core_options) function.
pub trait CoreOptions {
    /// Used to tell the frontend any options / settings your core supports.
    /// This can be done by using either of the following functions:
    /// - [`SetEnvironmentContext::set_core_options_v2`]
    /// - [`SetEnvironmentContext::set_core_options_v2_intl`]
    /// - [`SetEnvironmentContext::set_core_options`]
    /// - [`SetEnvironmentContext::set_core_options_intl`]
    /// - [`SetEnvironmentContext::set_variables`]
    fn set_core_options(&self, _ctx: &SetEnvironmentContext) -> bool {
        true
    }
}

/// This trait defines the basic functions that every libretro core must implement.
/// See also [`retro_core!()`].
pub trait Core: CoreOptions {
    /// Returns static info about this core.
    fn get_info(&self) -> SystemInfo;

    /// Called when the frontend needs information about the
    /// audio and video timings and the video geometry.
    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info;

    /// Called when the frontend set a new environment callback.
    ///
    /// Guaranteed to be called before [`Core::on_init`].
    fn on_set_environment(&mut self, _initial: bool, _ctx: &mut SetEnvironmentContext) {
        // Do nothing
    }

    /// Called when the libretro API has been initialized.
    fn on_init(&mut self, _ctx: &mut InitContext);

    /// Called when the libretro API gets destucted.
    fn on_deinit(&mut self, _ctx: &mut DeinitContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_set_controller_port_device(
        &mut self,
        _port: std::os::raw::c_uint,
        _device: std::os::raw::c_uint,
    ) {
        // Do nothing
    }

    /// Called when the frontend requests resetting the system.
    fn on_reset(&mut self, _ctx: &mut ResetContext) {
        // Do nothing
    }

    /// Called once per frame
    ///
    /// If a frame is not rendered for reasons where a game "dropped" a frame,
    /// this still counts as a frame, and [`Core::on_run`] should explicitly dupe
    /// a frame if [`environment::can_dupe`] returns [`true`].
    /// In this case, the video callback can take a NULL argument for data.
    fn on_run(&mut self, _ctx: &mut RunContext, _delta_us: Option<i64>) {
        // Do nothing
    }

    /// Returns the amount of data the implementation requires to serialize
    /// internal state (save states).
    ///
    /// Between calls to [`Core::on_load_game`] and [`Core::on_unload_game`], the
    /// returned size is never allowed to be larger than a previous returned
    /// value, to ensure that the frontend can allocate a save state buffer once.
    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> size_t {
        // Tell the frontend that we don’t support serialization
        0
    }

    /// Serializes internal state. If failed, or size is lower than
    /// [`Core::get_serialize_size`], it should return [`false`], [`true'] otherwise.
    fn on_serialize(&mut self, _slice: &mut [u8], _ctx: &mut SerializeContext) -> bool {
        // Tell the frontend that we don’t support serialization
        false
    }

    /// Deserializes internal state.
    ///
    /// **TODO:** Documentation
    fn on_unserialize(&mut self, _slice: &mut [u8], _ctx: &mut UnserializeContext) -> bool {
        // Tell the frontend that we don’t support serialization
        false
    }

    /// Called when a game should be loaded.
    /// Return [`true`] to indicate successful loading and [`false`] to indicate load failure.
    fn on_load_game(&mut self, _game: Option<retro_game_info>, _ctx: &mut LoadGameContext) -> bool {
        // By default we pretend that loading was successful
        true
    }

    /// Loads a "special" kind of game. Should not be used, except in extreme cases.
    ///
    /// **TODO:** Better documentation. What’s a “special” game?
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
    fn on_unload_game(&mut self, _ctx: &mut UnloadGameContext) {
        // Do nothing
    }

    /// Instructs the core to remove all applied cheats.
    fn on_cheat_reset(&mut self, _ctx: &mut CheatResetContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_cheat_set(
        &mut self,
        _index: std::os::raw::c_uint,
        _enabled: bool,
        _code: &CStr,
        _ctx: &mut CheatSetContext,
    ) {
        // Do nothing
    }

    /// Gets the region of the game.
    ///
    /// Can be any of:
    /// - [`RETRO_REGION_NTSC`]
    /// - [`RETRO_REGION_PAL`]
    fn on_get_region(&mut self, _ctx: &mut GetRegionContext) -> std::os::raw::c_uint {
        RETRO_REGION_NTSC
    }

    /// **TODO:** Documentation
    fn get_memory_data(
        &mut self,
        _id: std::os::raw::c_uint,
        _ctx: &mut GetMemoryDataContext,
    ) -> *mut std::os::raw::c_void {
        // Tell the frontend that we don’t support direct memory access
        std::ptr::null_mut()
    }

    /// **TODO:** Documentation
    fn get_memory_size(
        &mut self,
        _id: std::os::raw::c_uint,
        _ctx: &mut GetMemorySizeContext,
    ) -> size_t {
        // Tell the frontend that we don’t support direct memory access
        0
    }

    /// Gets called when the core options have been changed.
    ///
    /// Options get checked before [`Core::on_load_game`], [`Core::on_load_game_special`] and before each call of [`Core::on_run`].
    fn on_options_changed(&mut self, _ctx: &mut OptionsChangedContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_keyboard_event(
        &mut self,
        _down: bool,
        _keycode: retro_key,
        _character: u32,
        _key_modifiers: retro_mod,
    ) {
        // Do nothing
    }

    /// Called when the frontend needs more audio frames
    fn on_write_audio(&mut self, _ctx: &mut AudioContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_audio_set_state(&mut self, _enabled: bool) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_audio_buffer_status(&mut self, _active: bool, _occupancy: u32, _underrun_likely: bool) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_hw_context_reset(&mut self) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_hw_context_destroyed(&mut self) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_get_proc_address(&mut self, _symbol_name: &CStr) -> retro_proc_address_t {
        None
    }

    /// **TODO:** Documentation
    fn on_location_lifetime_status_initialized(&mut self, _ctx: &mut GenericContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_location_lifetime_status_deinitialized(&mut self, _ctx: &mut GenericContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_camera_initialized(&mut self, _ctx: &mut GenericContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_camera_deinitialized(&mut self, _ctx: &mut GenericContext) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_camera_raw_framebuffer(
        &mut self,
        _buffer: &[u32],
        _width: u32,
        _height: u32,
        _pitch: usize,
    ) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_camera_gl_texture(
        &mut self,
        _texture_id: u32,
        _texture_target: u32,
        _affine_matrix: &[f32; 3 * 3],
    ) {
        // Do nothing
    }

    /// **TODO:** Documentation
    fn on_set_eject_state(&mut self, _ejected: bool) -> bool {
        false
    }

    /// **TODO:** Documentation
    fn on_get_eject_state(&mut self) -> bool {
        false
    }

    /// **TODO:** Documentation
    fn on_get_image_index(&mut self) -> u32 {
        0
    }

    /// **TODO:** Documentation
    fn on_set_image_index(&mut self, _index: u32) -> bool {
        false
    }

    /// **TODO:** Documentation
    fn on_get_num_images(&mut self) -> u32 {
        0
    }

    /// **TODO:** Documentation
    fn on_replace_image_index(&mut self, _index: u32, _info: *const retro_game_info) -> bool {
        false
    }

    /// **TODO:** Documentation
    fn on_add_image_index(&mut self) -> bool {
        false
    }

    /// **TODO:** Documentation
    fn on_set_initial_image(&mut self, _index: u32, _path: &CStr) -> bool {
        false
    }

    /// **TODO:** Documentation
    fn on_get_image_path(&mut self, _index: u32) -> Option<CString> {
        None
    }

    /// **TODO:** Documentation
    fn on_get_image_label(&mut self, _index: u32) -> Option<CString> {
        None
    }

    /// **TODO:** Documentation
    fn on_core_options_update_display(&mut self) -> bool {
        false
    }
}
