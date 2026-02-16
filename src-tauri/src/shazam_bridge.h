#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
  SHAZAM_BRIDGE_EVENT_MATCH = 1,
  SHAZAM_BRIDGE_EVENT_NO_MATCH = 2,
  SHAZAM_BRIDGE_EVENT_ERROR = 3,
};

typedef void (*shazam_bridge_callback_t)(
    int32_t event_type,
    const char *title,
    const char *artist,
    const char *artwork_url,
    const char *apple_music_url,
    const char *web_url,
    const char *error_message,
    void *user_data);

void *shazam_bridge_create(shazam_bridge_callback_t callback,
                           void *user_data,
                           char **error_out);

bool shazam_bridge_start(void *bridge, char **error_out);

bool shazam_bridge_feed(void *bridge,
                        const float *samples,
                        uint32_t frame_count,
                        uint32_t channels,
                        double sample_rate,
                        char **error_out);

void shazam_bridge_stop(void *bridge);

void shazam_bridge_destroy(void *bridge);

void shazam_bridge_free_error(char *error_message);

#ifdef __cplusplus
}
#endif
