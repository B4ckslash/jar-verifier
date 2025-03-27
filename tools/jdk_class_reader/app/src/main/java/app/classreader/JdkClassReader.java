/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

package app.classreader;

import java.io.*;
import java.lang.reflect.Executable;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.*;
import java.util.stream.Collectors;

import static java.util.Arrays.stream;

public class JdkClassReader {

    public static final String SEPARATOR = ":";

    private static final ModuleLayer BOOT = ModuleLayer.boot();

    public static void main(String[] args) {
        final String modulePath = args[0];
        final String outputPath = args[1];

        try (final OutputStreamWriter writer = new OutputStreamWriter(new FileOutputStream(outputPath))) {
            final List<String> classes = getClassNamesFromModuleFile(modulePath);
            writeClassInfo(classes, writer);
        } catch (final IOException e) {
            e.printStackTrace();
        }
    }

    private static void writeClassInfo(List<String> classes, OutputStreamWriter writer) throws IOException {
        Class<?> clazz;
        for (final String className : classes) {
            try {
                clazz = Class.forName(className.replace('/', '.'));
            } catch (ClassNotFoundException e) {
                System.err.println("Class not found: " + className + "! Skipping...");
                continue;
            } catch (Throwable t) {
                System.err.println("Failed to load class " + className + ": " + t);
                continue;
            }
            if (!clazz.getModule().isExported(clazz.getPackageName())
                    || !(Modifier.isPublic(clazz.getModifiers())
                    || Modifier.isProtected(clazz.getModifiers()))) {
                continue;
            }
            final Class<?> superClass = clazz.getSuperclass();
            final List<String> constructors = Arrays.stream(clazz.getDeclaredConstructors())
                    .filter(constructor -> Modifier.isPublic(constructor.getModifiers()) || Modifier.isProtected(constructor.getModifiers()))
                    .map(c -> String.format("--%s%n", getInternalRepresentation(c))).toList();
            final List<String> methods = Arrays.stream(clazz.getDeclaredMethods())
                    .filter(method -> Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers()))
                    .map(m -> String.format("--%s%n", getInternalRepresentation(m))).toList();
            writer.write(className);
            if (superClass != null) {
                writer.write(SEPARATOR + superClass.getName().replace('.', '/'));
            } else {
                writer.write(SEPARATOR + "null");
            }
            writer.write(SEPARATOR + (constructors.size() + methods.size()));
            writer.write(System.lineSeparator());
            for (final String constructor : constructors) {
                writer.write(constructor);
            }
            for (final String method : methods) {
                writer.write(method);
            }
        }
    }

    private static String getPackageName(final String className) {
        return className.substring(0, className.lastIndexOf('/'));
    }

    private static List<String> getClassNamesFromModuleFile(String modulePath) throws IOException {
        final Process jimage = new ProcessBuilder("jimage", "list", modulePath).start();
        final BufferedReader reader = new BufferedReader(new InputStreamReader(jimage.getInputStream()));
        final List<String> classes = new ArrayList<>(10_000);
        String line = reader.readLine();
        while (line != null) {
            if (line.startsWith("Module: ")) {
                final String moduleName = line.split(" ")[1].trim();
                final Optional<Module> moduleOptional = BOOT.findModule(moduleName);
                if (moduleOptional.isPresent()) {
                    final Module module = moduleOptional.get();

                    String contentLine = reader.readLine();
                    while (contentLine != null && !contentLine.startsWith("Module: ")) {
                        if (contentLine.endsWith(".class") && !contentLine.contains("module-info")) {
                            final String packageName = getPackageName(contentLine).trim();
                            if (module.isExported(packageName.replace('/', '.'))) {
                                classes.add(contentLine.replace(".class", "").trim());
                            }
                        }
                        contentLine = reader.readLine();
                    }
                    line = contentLine;
                    continue;
                }
            }
            line = reader.readLine();
        }
        return classes;
    }

    private static String getInternalRepresentation(final Executable executable) {
        final String name = executable instanceof Method ? executable.getName() : "<init>";
        final String parameters = stream(executable.getParameterTypes())
                .map(JdkClassReader::mapType)
                .collect(Collectors.joining());
        final String returnType = executable instanceof final Method m ? mapType(m.getReturnType()) : mapType(void.class);
        return String.format("%s(%s)%s", name, parameters, returnType);
    }

    /**
     * See <a href="https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.3.2">the class reference</a>
     *
     * @param type A class representing the type to be mapped
     * @return A JVM representation of that type
     */
    private static String mapType(final Class<?> type) {
        if (byte.class.equals(type)) {
            return "B";
        }
        if (char.class.equals(type)) {
            return "C";
        }
        if (double.class.equals(type)) {
            return "D";
        }
        if (float.class.equals(type)) {
            return "F";
        }
        if (int.class.equals(type)) {
            return "I";
        }
        if (long.class.equals(type)) {
            return "J";
        }
        if (short.class.equals(type)) {
            return "S";
        }
        if (boolean.class.equals(type)) {
            return "Z";
        }
        if (void.class.equals(type)) {
            return "V";
        }
        if (type.isArray()) {
            return type.getName().replace('.', '/');
        }
        return "L" + type.getName().replace('.', '/') + ";";
    }
}
