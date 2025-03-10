package app.classreader;

import java.io.*;
import java.lang.reflect.Executable;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;

import static java.util.Arrays.stream;

public class JdkClassReader {
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
        for (final String className : classes) {
            try {
                writer.write(className + System.lineSeparator());

                final Class<?> clazz = Class.forName(className.replace('/', '.'));
                final List<String> constructors = Arrays.stream(clazz.getDeclaredConstructors())
                        .filter(constructor -> Modifier.isPublic(constructor.getModifiers()) || Modifier.isProtected(constructor.getModifiers()))
                        .map(c -> String.format("--%s%n", getInternalRepresentation(c))).toList();
                final List<String> methods = Arrays.stream(clazz.getDeclaredMethods())
                        .filter(method -> Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers()))
                        .map(m -> String.format("--%s%n", getInternalRepresentation(m))).toList();
                for (final String constructor : constructors) {
                    writer.write(constructor);
                }
                for (final String method : methods) {
                    writer.write(method);
                }
            } catch (ClassNotFoundException e) {
                System.err.println("Class not found: " + className);
            }
        }
    }

    private static List<String> getClassNamesFromModuleFile(String modulePath) throws IOException {
        final Process jimage = new ProcessBuilder("jimage", "list", modulePath).start();
        final BufferedReader reader = new BufferedReader(new InputStreamReader(jimage.getInputStream()));
        return reader.lines()
                .map(String::trim)
                .filter(line -> line.endsWith(".class") && line.startsWith("java"))
                .map(line -> line.replace(".class", ""))
                .toList();
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
            return type.getName();
        }
        return "L" + type.getName().replace('.', '/') + ";";
    }
}
