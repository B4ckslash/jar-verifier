/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

package org.example;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class LibraryWithoutDependenciesTest {

    @Test
    void areObjectsEqual_notEqual() {
        final Object o1 = new Object();
        final Object o2 = new Object();
        assertFalse(LibraryWithoutDependencies.areObjectsEqual(o1, o2));
    }

    @Test
    void areObjectsEqual_equal() {
        final String s1 = "test";
        final String s2 = "test";
        assertTrue(LibraryWithoutDependencies.areObjectsEqual(s1, s2));
    }

    @Test
    void copyOfRange() {
        final String s1 = "test";
        final char[] compare = new char[]{'e', 's'};
        final char[] result = LibraryWithoutDependencies.copyOfRange(s1.toCharArray(), 1, 3);
        assertArrayEquals(compare, result);
    }

    @Test
    void deepCopy() {
        final String s1 = "test1";
        final String s2 = "test2";
        final String s3 = "test3";
        final String[][] compare = new String[][]{{s1, s2}, {s3}};
        final String[][] result = LibraryWithoutDependencies.deepCopy(compare);
        assertArrayEquals(compare, result);
    }

    @Test
    void testThreadGroupSuspension() {
        LibraryWithoutDependencies.suspendThreadGroup_removedInJava21();
    }
}
