/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

package org.example;

public class LibraryWithoutDependencies {
    public static boolean areObjectsEqual(Object o1, Object o2) {
        return o1.equals(o2);
    }

    public static char[] copyOfRange(char[] original, int start, int end) {
        final char[] array = new char[end - start];
        System.arraycopy(original, start, array, 0, end - start);
        return array;
    }

    public static String[][] deepCopy(String[][] original) {
        final String[][] copy = new String[original.length][];
        for (int i = 0; i < original.length; i++) {
           copy[i] = original[i].clone();
        }

        return copy;
    }
}
