// Turns a booted iOS simulator to portrait or landscape without the Simulator app's window [a headless sim has no
// menu]: one GraphicsServices orientation event [type 50] to the device's PurpleWorkspacePort, laid out as the
// Simulator app's own Device > Rotate sends it [Xcode 26]. Objective-C: it messages CoreSimulator's private classes.
// Build: clang -fobjc-arc -framework Foundation tests/ios/rotate.m -o <dir>/rotate . Run: rotate <udid> <1-4>
#import <Foundation/Foundation.h>
#import <dlfcn.h>
#import <mach/mach.h>

@interface NSObject (CoreSimulatorSPI)
+ (id)sharedServiceContextForDeveloperDir:(NSString *)dir error:(NSError **)error;
- (id)defaultDeviceSetWithError:(NSError **)error;
- (NSDictionary *)devicesByUDID;
- (unsigned int)lookup:(NSString *)name error:(NSError **)error;
@end

int main(int argc, char **argv) {
  @autoreleasepool {
    if (argc != 3) {
      fprintf(stderr, "usage: rotate <udid> <1 portrait | 2 upside down | 3 landscape left | 4 landscape right>\n");
      return 2;
    }
    if (!dlopen("/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator", RTLD_NOW)) {
      fprintf(stderr, "rotate: load CoreSimulator: %s\n", dlerror());
      return 1;
    }
    NSString *dev = NSProcessInfo.processInfo.environment[@"DEVELOPER_DIR"] ?: @"/Applications/Xcode.app/Contents/Developer";
    NSError *err = nil;
    id ctx = [NSClassFromString(@"SimServiceContext") sharedServiceContextForDeveloperDir:dev error:&err];
    id set = [ctx defaultDeviceSetWithError:&err];
    NSUUID *udid = [[NSUUID alloc] initWithUUIDString:[NSString stringWithUTF8String:argv[1]]];
    id sim = udid ? [set devicesByUDID][udid] : nil;
    if (!sim) { fprintf(stderr, "rotate: no simulator %s %s\n", argv[1], err.description.UTF8String ?: ""); return 1; }
    unsigned int port = [sim lookup:@"PurpleWorkspacePort" error:&err];
    if (!port) { fprintf(stderr, "rotate: %s has no PurpleWorkspacePort [not booted?] %s\n", argv[1], err.description.UTF8String ?: ""); return 1; }

    uint32_t msg[32] = {0};
    msg[0] = MACH_MSG_TYPE_COPY_SEND;  // msgh_bits
    msg[1] = (4 + 0x6b) & ~3u;         // msgh_size, as the Simulator app computes it for a 4-byte payload
    msg[2] = port;                     // msgh_remote_port
    msg[5] = 0x7b;                     // msgh_id
    msg[6] = 50 | 0x20000;             // event type: device orientation changed, with the flag the Simulator app adds
    msg[18] = 4;                       // payload size
    msg[19] = (uint32_t)atoi(argv[2]); // the orientation
    kern_return_t kr = mach_msg_send((mach_msg_header_t *)msg);
    if (kr != KERN_SUCCESS) { fprintf(stderr, "rotate: send to %s: 0x%x\n", argv[1], kr); return 1; }
    printf("rotate: %s -> %s\n", argv[1], argv[2]);
  }
  return 0;
}
