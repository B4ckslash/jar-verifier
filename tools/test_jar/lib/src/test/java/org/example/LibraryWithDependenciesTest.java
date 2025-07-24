/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

package org.example;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class LibraryWithDependenciesTest {

    public static final String TEST_PREFIX = "TestPrefix";
    LibraryWithDependencies library;

    @BeforeEach
    void setUp() {
        library = new LibraryWithDependencies(TEST_PREFIX);
    }

    @Test
    void testGenerateNewId() {
        final String newId = library.generateNewId();
        assertNotNull(newId);
        assertTrue(newId.startsWith(TEST_PREFIX));
    }
}
