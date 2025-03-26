/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

package org.example;

import org.apache.commons.math3.random.MersenneTwister;
import org.apache.commons.math3.random.UniformRandomGenerator;

public class LibraryWithDependencies {
    public static final UniformRandomGenerator RAND = new UniformRandomGenerator(new MersenneTwister());
    private final String prefix;

    public LibraryWithDependencies() {
        this(Double.toHexString(generateRandomDouble()));
    }

    public LibraryWithDependencies(final String prefix) {
        this.prefix = prefix;
    }

    private static double generateRandomDouble() {
        return RAND.nextNormalizedDouble();
    }

    public String generateNewId()
    {
        return prefix + "--" + Double.toHexString(generateRandomDouble());
    }
}
