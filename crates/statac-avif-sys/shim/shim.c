#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "avif/avif.h"

static void set_error(char *error, size_t error_len, avifResult result, const char *detail) {
    if (detail && detail[0]) {
        snprintf(error, error_len, "%s: %s", avifResultToString(result), detail);
    } else {
        snprintf(error, error_len, "%s", avifResultToString(result));
    }
}

/* Encodes 8-bit RGB or straight RGBA. On failure, writes a terminated
   message into error, which must hold at least one byte. */
int avif_shim_encode(const uint8_t *pixels, uint32_t width, uint32_t height,
                     uint32_t row_bytes, int has_alpha, int speed, int quality,
                     int quality_alpha, int threads, uint8_t **out, size_t *out_len,
                     char *error, size_t error_len) {
    *out = NULL;
    *out_len = 0;
    error[0] = '\0';

    avifImage *image = avifImageCreate(width, height, 8, AVIF_PIXEL_FORMAT_YUV444);
    if (!image) {
        set_error(error, error_len, AVIF_RESULT_OUT_OF_MEMORY, NULL);
        return AVIF_RESULT_OUT_OF_MEMORY;
    }
    image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
    image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB;
    image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_BT601;
    image->yuvRange = AVIF_RANGE_FULL;

    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, image);
    rgb.format = has_alpha ? AVIF_RGB_FORMAT_RGBA : AVIF_RGB_FORMAT_RGB;
    rgb.depth = 8;
    rgb.alphaPremultiplied = AVIF_FALSE;
    /* libavif only reads from rgb.pixels during conversion. */
    rgb.pixels = (uint8_t *)pixels;
    rgb.rowBytes = row_bytes;

    avifResult result = avifImageRGBToYUV(image, &rgb);
    if (result != AVIF_RESULT_OK) {
        set_error(error, error_len, result, NULL);
        avifImageDestroy(image);
        return result;
    }

    avifEncoder *encoder = avifEncoderCreate();
    if (!encoder) {
        set_error(error, error_len, AVIF_RESULT_OUT_OF_MEMORY, NULL);
        avifImageDestroy(image);
        return AVIF_RESULT_OUT_OF_MEMORY;
    }
    encoder->codecChoice = AVIF_CODEC_CHOICE_AOM;
    encoder->speed = speed;
    encoder->quality = quality;
    encoder->qualityAlpha = quality_alpha;
    encoder->maxThreads = threads;

    avifRWData output = AVIF_DATA_EMPTY;
    result = avifEncoderWrite(encoder, image, &output);
    if (result != AVIF_RESULT_OK) {
        set_error(error, error_len, result, encoder->diag.error);
        avifRWDataFree(&output);
    } else {
        *out = output.data;
        *out_len = output.size;
    }
    avifEncoderDestroy(encoder);
    avifImageDestroy(image);
    return result;
}

void avif_shim_free(uint8_t *data, size_t len) {
    avifRWData output = { data, len };
    avifRWDataFree(&output);
}

void avif_shim_codec_versions(char *out, size_t len) {
    if (len == 0) {
        return;
    }
    char buffer[256];
    avifCodecVersions(buffer);
    strncpy(out, buffer, len - 1);
    out[len - 1] = '\0';
}
