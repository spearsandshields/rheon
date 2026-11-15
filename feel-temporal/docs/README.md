# FEEL temporal

## Overview

FEEL temporal types used by [**dsntk**](https://crates.io/crates/dsntk) crate.

Every constructor and every arithmetic step that can leave the supported range returns `Option`;
no operation calls a chrono operation that can panic.
