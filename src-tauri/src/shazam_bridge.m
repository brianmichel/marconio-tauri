#import "shazam_bridge.h"

#import <AVFoundation/AVFoundation.h>
#import <Foundation/Foundation.h>
#import <ShazamKit/ShazamKit.h>

#include <stdlib.h>
#include <string.h>

static char *copy_error(NSString *message) {
  const char *utf8 = message.UTF8String;
  if (utf8 == NULL) {
    utf8 = "Unknown error";
  }
  return strdup(utf8);
}

static const char *optional_cstring(NSString *value) {
  if (value == nil) {
    return NULL;
  }
  return value.UTF8String;
}

@interface MarconioShazamBridge : NSObject <SHSessionDelegate>

@property(nonatomic, assign) shazam_bridge_callback_t callback;
@property(nonatomic, assign) void *userData;
@property(nonatomic, strong) SHSession *session;
@property(nonatomic, assign) BOOL active;

- (instancetype)initWithCallback:(shazam_bridge_callback_t)callback
                        userData:(void *)userData;
- (BOOL)startWithError:(NSError **)error;
- (BOOL)feedSamples:(const float *)samples
         frameCount:(uint32_t)frameCount
           channels:(uint32_t)channels
         sampleRate:(double)sampleRate
              error:(NSError **)error;
- (void)stop;

@end

@implementation MarconioShazamBridge

- (instancetype)initWithCallback:(shazam_bridge_callback_t)callback
                        userData:(void *)userData {
  self = [super init];
  if (self == nil) {
    return nil;
  }
  _callback = callback;
  _userData = userData;
  _session = nil;
  _active = NO;
  return self;
}

- (BOOL)startWithError:(NSError **)error {
  if (@available(macOS 12.0, *)) {
    @synchronized(self) {
      [self stop];
      self.session = [[SHSession alloc] init];
      self.session.delegate = self;
      self.active = YES;
    }
    return YES;
  }

  if (error != NULL) {
    *error = [NSError errorWithDomain:@"Marconio.Shazam"
                                 code:1
                             userInfo:@{
                               NSLocalizedDescriptionKey :
                                   @"ShazamKit requires macOS 12 or later."
                             }];
  }
  return NO;
}

- (BOOL)feedSamples:(const float *)samples
         frameCount:(uint32_t)frameCount
           channels:(uint32_t)channels
         sampleRate:(double)sampleRate
              error:(NSError **)error {
  if (samples == NULL) {
    return YES;
  }
  if (frameCount == 0 || channels == 0) {
    return YES;
  }

  SHSession *session = nil;
  BOOL active = NO;
  @synchronized(self) {
    session = self.session;
    active = self.active;
  }

  if (!active || session == nil) {
    return YES;
  }

  if (@available(macOS 12.0, *)) {
    AVAudioFormat *format = [[AVAudioFormat alloc]
        initWithCommonFormat:AVAudioPCMFormatFloat32
                   sampleRate:sampleRate
                     channels:(AVAudioChannelCount)channels
                  interleaved:YES];

    if (format == nil) {
      if (error != NULL) {
        *error = [NSError
            errorWithDomain:@"Marconio.Shazam"
                       code:2
                   userInfo:@{
                     NSLocalizedDescriptionKey :
                         @"Unable to create audio format for ShazamKit."
                   }];
      }
      return NO;
    }

    AVAudioPCMBuffer *buffer =
        [[AVAudioPCMBuffer alloc] initWithPCMFormat:format
                                      frameCapacity:(AVAudioFrameCount)frameCount];

    if (buffer == nil) {
      if (error != NULL) {
        *error = [NSError
            errorWithDomain:@"Marconio.Shazam"
                       code:3
                   userInfo:@{
                     NSLocalizedDescriptionKey :
                         @"Unable to create PCM buffer for ShazamKit."
                   }];
      }
      return NO;
    }

    buffer.frameLength = (AVAudioFrameCount)frameCount;
    AudioBufferList *audioBufferList = buffer.mutableAudioBufferList;
    if (audioBufferList == NULL || audioBufferList->mNumberBuffers < 1 ||
        audioBufferList->mBuffers[0].mData == NULL) {
      if (error != NULL) {
        *error = [NSError
            errorWithDomain:@"Marconio.Shazam"
                       code:4
                   userInfo:@{
                     NSLocalizedDescriptionKey :
                         @"PCM buffer did not expose writable audio data."
                   }];
      }
      return NO;
    }

    size_t sampleCount = (size_t)frameCount * (size_t)channels;
    size_t byteCount = sampleCount * sizeof(float);
    memcpy(audioBufferList->mBuffers[0].mData, samples, byteCount);
    audioBufferList->mBuffers[0].mDataByteSize = (UInt32)byteCount;

    [session matchStreamingBuffer:buffer atTime:nil];
    return YES;
  }

  if (error != NULL) {
    *error = [NSError errorWithDomain:@"Marconio.Shazam"
                                 code:5
                             userInfo:@{
                               NSLocalizedDescriptionKey :
                                   @"ShazamKit requires macOS 12 or later."
                             }];
  }
  return NO;
}

- (void)stop {
  @synchronized(self) {
    self.active = NO;
    self.session.delegate = nil;
    self.session = nil;
  }
}

- (void)session:(SHSession *)session didFindMatch:(SHMatch *)match
    API_AVAILABLE(macos(12.0)) {
  BOOL shouldEmit = NO;
  @synchronized(self) {
    if (self.active) {
      shouldEmit = YES;
      self.active = NO;
    }
  }

  if (!shouldEmit || self.callback == NULL) {
    [self stop];
    return;
  }

  SHMatchedMediaItem *item = match.mediaItems.firstObject;
  self.callback(SHAZAM_BRIDGE_EVENT_MATCH, optional_cstring(item.title),
                optional_cstring(item.artist),
                optional_cstring(item.artworkURL.absoluteString),
                optional_cstring(item.appleMusicURL.absoluteString),
                optional_cstring(item.webURL.absoluteString), NULL, self.userData);
  [self stop];
}

- (void)session:(SHSession *)session
    didNotFindMatchForSignature:(SHSignature *)signature
                           error:(NSError *)error API_AVAILABLE(macos(12.0)) {
  BOOL shouldEmit = NO;
  @synchronized(self) {
    if (self.active) {
      shouldEmit = YES;
      self.active = NO;
    }
  }

  if (!shouldEmit || self.callback == NULL) {
    [self stop];
    return;
  }

  if (error != nil) {
    self.callback(SHAZAM_BRIDGE_EVENT_ERROR, NULL, NULL, NULL, NULL, NULL,
                  optional_cstring(error.localizedDescription), self.userData);
  } else {
    self.callback(SHAZAM_BRIDGE_EVENT_NO_MATCH, NULL, NULL, NULL, NULL, NULL,
                  NULL, self.userData);
  }
  [self stop];
}

@end

void *shazam_bridge_create(shazam_bridge_callback_t callback, void *user_data,
                           char **error_out) {
  @autoreleasepool {
    if (callback == NULL) {
      if (error_out != NULL) {
        *error_out = copy_error(@"Shazam callback cannot be null.");
      }
      return NULL;
    }

    MarconioShazamBridge *bridge =
        [[MarconioShazamBridge alloc] initWithCallback:callback
                                              userData:user_data];
    if (bridge == nil) {
      if (error_out != NULL) {
        *error_out = copy_error(@"Unable to create Shazam bridge.");
      }
      return NULL;
    }
    return (__bridge_retained void *)bridge;
  }
}

bool shazam_bridge_start(void *bridge, char **error_out) {
  if (bridge == NULL) {
    if (error_out != NULL) {
      *error_out = copy_error(@"Shazam bridge is not initialized.");
    }
    return false;
  }

  @autoreleasepool {
    MarconioShazamBridge *typed = (__bridge MarconioShazamBridge *)bridge;
    NSError *error = nil;
    BOOL ok = [typed startWithError:&error];
    if (!ok && error_out != NULL) {
      *error_out = copy_error(error.localizedDescription);
    }
    return ok;
  }
}

bool shazam_bridge_feed(void *bridge, const float *samples, uint32_t frame_count,
                        uint32_t channels, double sample_rate,
                        char **error_out) {
  if (bridge == NULL) {
    if (error_out != NULL) {
      *error_out = copy_error(@"Shazam bridge is not initialized.");
    }
    return false;
  }

  @autoreleasepool {
    MarconioShazamBridge *typed = (__bridge MarconioShazamBridge *)bridge;
    NSError *error = nil;
    BOOL ok = [typed feedSamples:samples
                      frameCount:frame_count
                        channels:channels
                      sampleRate:sample_rate
                           error:&error];
    if (!ok && error_out != NULL) {
      *error_out = copy_error(error.localizedDescription);
    }
    return ok;
  }
}

void shazam_bridge_stop(void *bridge) {
  if (bridge == NULL) {
    return;
  }

  @autoreleasepool {
    MarconioShazamBridge *typed = (__bridge MarconioShazamBridge *)bridge;
    [typed stop];
  }
}

void shazam_bridge_destroy(void *bridge) {
  if (bridge == NULL) {
    return;
  }

  @autoreleasepool {
    MarconioShazamBridge *typed =
        (__bridge_transfer MarconioShazamBridge *)bridge;
    [typed stop];
  }
}

void shazam_bridge_free_error(char *error_message) {
  if (error_message != NULL) {
    free(error_message);
  }
}
