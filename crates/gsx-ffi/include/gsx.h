#ifndef GSX_H
#define GSX_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define GSX_MEDIA_NONE 0

#define GSX_MEDIA_IMAGE 1

#define GSX_MEDIA_VIDEO 2

#define GSX_MEDIA_GIF 3

#define GSX_MEDIA_AUDIO 4

#define GSX_PLACEMENT_NONE 0

#define GSX_PLACEMENT_POST 1

#define GSX_PLACEMENT_REEL 2

#define GSX_PLACEMENT_STORY 3

#define GSX_FORMAT_IMAGE 0

#define GSX_FORMAT_GIF 1

#define GSX_FORMAT_VIDEO 2

#define GSX_FORMAT_AUDIO 3

typedef struct gsx_error gsx_error;

typedef struct gsx_output gsx_output;

typedef struct gsx_media {
  const uint8_t *id;
  size_t id_len;
  const uint8_t *cover;
  size_t cover_len;
} gsx_media;

typedef struct gsx_media_fact {
  const uint8_t *id;
  size_t id_len;
  uint32_t format;
  const uint8_t *alt;
  size_t alt_len;
  bool spoiler;
  bool inline_;
} gsx_media_fact;

typedef struct gsx_facts {
  uint32_t media;
  size_t media_count;
  uint32_t placement;
  const uint8_t *title;
  size_t title_len;
  const uint8_t *target_url;
  size_t target_url_len;
  int64_t max_characters;
  int64_t characters_reserved_per_url;
  int64_t caption_limit;
  int64_t description_limit;
  bool entities_allowed;
  const struct gsx_media_fact *items;
  size_t items_len;
} gsx_facts;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

struct gsx_output *gsx_parse(const uint8_t *text,
                             size_t len,
                             const struct gsx_media *media,
                             size_t media_len,
                             struct gsx_error **error);

struct gsx_output *gsx_serialize(const uint8_t *text,
                                 size_t len,
                                 const struct gsx_media *media,
                                 size_t media_len,
                                 struct gsx_error **error);

struct gsx_output *gsx_project(const uint8_t *text,
                               size_t len,
                               const struct gsx_media *media,
                               size_t media_len,
                               const uint8_t *target,
                               size_t target_len,
                               const struct gsx_facts *facts,
                               struct gsx_error **error);

struct gsx_output *gsx_targets(struct gsx_error **error);

const char *gsx_output_json(const struct gsx_output *output, size_t *len);

void gsx_output_free(struct gsx_output *output);

const char *gsx_error_message(const struct gsx_error *error);

void gsx_error_free(struct gsx_error *error);

const char *gsx_version(void);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* GSX_H */
