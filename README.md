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
`cargo test` call for running tests. It works on both Linux and Windows 64bit;
on Windows, you'll need to use the `x86_64-pc-windows-msvc` target which in
turn required you to install a few GB of MSVC build tools.

### Using it
Best is to declare it as a dependency of your project via git.

### Caveats if you're working on this and get f8**ed by Windows
MSVC won't link against a .dll and then later load it dynamically, but it also
won't "blindly" link against a shared object and check if everything is dandy
at runtime. Well, sort of, because the resource it needs to "link" is just a
file containing a bunch of obfuscated symbol names and then they still check
again at runtime (because, well, there really isn't any other way).

It is what it is, whether I like it (I don't) or not (I really don't) isn't
going to change anything. The good news is that if you have the .dll, you can
actually just generate this obfuscated-symbols-file using Microsoft's own
tooling.

To link against the AiM library on Windows, you need a .lib file corresponding
to the .dll file they provide. There is one included in the `aim` directory,
but if for whatever reason you need a newer one, read one.

You should already have the MSVC build tools installed to work with Rust's
`x86_64-pc-windows-msvc` target. Go ahead and fire up your Visual Studio
installer again and install the C++ CLI tools, or the Windows Foundational
tools, or whatever your version of Visual Studio calls this stuff; if unsure
just [Google "visual studio
dumpbin"](https://www.google.com/search?q=visual+studio+dumpbin) or some
similar combination of random terms and go from there.

Now that you've installed another 2GB or so of stuff you probably won't ever
need again, do the following:

1. From Start menu run "Visual Studio Command Prompt".
1. Navigate to `where/you/put/xdrkrs/aim`
1. Execute command:
    `dumpbin /exports libxdrk-x86_64.dll > libxdrk-x86_64.def`
    This command prints some information about given DLL library in textual
    form to its standard output. We redirect it to a text file with DEF
    extension. But to make it real DEF file, we need to edit it.
1. Open `libxdrk-x86_64.def` in some text editor and edit it to contain only
    the names of exported functions in form of:
    ```
    EXPORTS
    function_1_name
    function_2_name
    function_3_name
    ```
    At this point you may also want to compare the DEF file with the one that
    is provided in the `aim` directory of this repo.
1. Execute another command:
    ```
    lib /def:libxdrk-x86_64.def /out:libxdrk-x86_64.lib /machine:x64
    ```

And there you have it! The so much required LIB file generated from DLL
library.

I stole this guide [from the
internet](https://asawicki.info/news_1420_generating_lib_file_for_dll_library).
As always, thank you, internet!
