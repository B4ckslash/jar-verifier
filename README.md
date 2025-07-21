# JAR Verifier

A utility to check Java JAR libraries for any unsatisfied dependencies by
checking their imported classes and used methods.

## Building

Debug build:

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

It is possible to embed information about Java SE libraries directly into the executable.
This can be useful if you can't (or don't want to) distribute this information alongside
the executable. To do this, simply append `-F embedded_classinfo` to the build command.

## Usage

```bash
jar_verifier [OPTIONS] <CLASSPATH> <JDK_CLASSINFO>
Arguments:
  <CLASSPATH>      Classpath of JARs to be checked
  <JDK_CLASSINFO>  A file listing the available classes and methods of the relevant JDK
Options:
  -t, --threads <THREADS>          The number of threads to use [default: 1]
  -o, --output-file <OUTPUT_FILE>  The output file path. Prints to stdout if not set
  -h, --help                       Print help
  -V, --version                    Print version
```

The `CLASSPATH` must be a list of `.jar` files, separated by a semicolon (`;`).

Shell and glob expansions are supported, so you can give a path like
`path/to/lib/dir/*.jar` or `~/path/to/jars/lib.jar`.
Globbing is done by the program itself, so any path with globs should be
put in single quotes (`'`), otherwise the shell will expand it itself and mess
up the input.

The output file given with the `-o` flag will be overwritten if it already exists.

The `JDK_CLASSINFO` argument is not available when the executable was compiled with
embedded class information.
A class information file for Java SE 17 is available in the `data/` directory.
This is also the one that gets embedded when the feature flag is specified
during build.

### Output

The output can be roughly described by the following grammar:

```
ClassRequirements := ClassName Requirement+
Requirement       := <TAB>(ClassImport|MethodImport)
ClassImport       := "Class " ClassName
MethodImport      := ("ClassMethod "|"IfaceMethod ") MethodSpec
MethodSpec        := ClassName "#" MethodName MethodDescriptor
```

`ClassName`, `MethodName` and `MethodDescriptor` are described by the JVM
class file format spec:
[Names](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.2)
[MethodDescriptor](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.3.3)

An example of the output can be found in the [test data](./testdata/requirements.txt).

## Creating .classinfo files

There is a tool to create `.classinfo` files in the `tools/jdk_class_reader` directory
which works by reading the module file of your JDK via the `jimage` program.

### Prerequisites

- JDK newer than Java 9

### Usage

```bash
cd tools/jdk_class_reader
./gradlew run --args 'path/to/module/file path/to/output/classinfo'

EXAMPLE:
./gradlew run --args '/usr/lib/jvm/java-17/lib/modules /tmp/17.classinfo'
```

The module file MUST be a JImage file. You can check this by running
`jimage verify <path>`.

Afterwards, you can find a file specifying the class information at
the given output path. To prevent any mismatches in classes, you may
want to adjust the Java version specified in `app/build.gradle.kts`
to align with the Java version that you want to process.

The above command will download a [Gradle](https://gradle.org/) distribution ZIP file
if you do not have Gradle installed.
