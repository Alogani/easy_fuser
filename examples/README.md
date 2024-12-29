# Easy Fuser Examples

This directory contains various examples demonstrating the usage of the Easy Fuser library. These examples are designed to be comprehensive and educational, showcasing different aspects and capabilities of Easy Fuser.

## Purpose

The examples in this folder are:
- Created for educational purposes
- Meant to be exhaustive rather than concise
- Focused on demonstrating filesystem implementations

## Important Notes

1. **Work in Progress**: These examples are continuously evolving and are not intended for production use. They serve as learning resources and demonstrations of Easy Fuser's capabilities.

2. **Focus on Filesystem Implementation**: When exploring these examples, pay special attention to the `filesystem.rs` file (when present) to understand the core usage of Easy Fuser.

3. **Comprehensive Examples**: Unlike minimal examples, these are designed to be thorough, showcasing various features and use cases of Easy Fuser.

4. **Testing and Reliability**: All examples are thoroughly tested before being pushed to the main branch, ensuring a level of reliability and correctness.

5. **Binary Availability**: For each GitHub release, binaries for all example files are included, allowing easy exploration without compilation.

## Contributing

We welcome contributions to improve existing examples or add new ones. If you have ideas for new examples or improvements to existing ones, feel free to contribute! Please ensure that your contributions maintain the educational focus of these examples.

## Exploring the Examples

To get the most out of these examples:
1. Start by examining the `filesystem.rs` file in each example (if present).
2. Run the examples and interact with the resulting filesystems.
3. Experiment with modifying the examples to understand how changes affect the filesystem behavior.
4. Activate logging using env-logger to gain more insights into the filesystem operations. This will provide detailed logging information, which can be invaluable for understanding the inner workings of the filesystem and for debugging purposes.

## Common Caveats

When working with these examples, be aware of the following:

1. **Handling Crashes and Interruptions**: If a program crashes or is stopped abruptly (e.g., using Ctrl+C), it may leave the mountpoint in an inconsistent state. This applies to all examples except those that explicitly include a handler for such cases.

2. **Proper Unmounting**: To properly unmount the filesystem and stop the program (or to resolve a bad state after a crash), use the following command:

`fusermount -u <mountpoint>`

This is the preferred method for both unmounting and resolving any issues with the mountpoint.

3. **Testing and Debugging**: When testing or debugging these examples, always ensure you properly unmount the filesystem before making changes or restarting the program.


Remember, these examples are meant to be learning tools. Don't hesitate to dive deep into the code, experiment, and learn from them!
