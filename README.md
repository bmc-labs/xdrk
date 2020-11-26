# `xdrkrs` - a library to access files produced by AiM devices

Or rather, a Rust wrapper for the shared library provided by AiM which is
written in C/C++ and provides an unsafe interface. This wrapper does its best
to make the interface safe, which it does - but it still contains one pain
point, which is having to work with write buffers (or, as you'd call them in C,
pointers to head-allocated arrays). However also this is wrapped as tight as it
can be.

Executive summary: safely use the AiM access library for XRK/DRK files.

### Build and test it
Well, chances are you know Rust, so you'll just be dealing with the usual
`cargo test` call for running tests. It works on both Linux and Windows 64bit.

### Using it
Best is to declare it as a dependency of your project via git.
