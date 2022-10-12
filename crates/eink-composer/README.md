
// SurfaceFlingerLib
// SurfaceFlinger

MS_00011_00_07D2_D6

SurfaceComposerClient
    create_connection 
    create_surface() -> SurfaceControl

SurfaceFlinger
    create_normal_surface_locked(
        const sp<Client>& client, 
        DisplayID display, 
        int32_t id, 
        uint32_t w, 
        uint32_t h, 
        uint32_t flags, 
        PixelFormat& format) -> sp<LayerBaseClient> {
