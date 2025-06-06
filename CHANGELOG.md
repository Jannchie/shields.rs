## HEAD

[v0.5.0...HEAD](https://github.com/Jannchie/shields.rs/compare/v0.5.0...HEAD)

## v0.5.0

[v0.4.0...v0.5.0](https://github.com/Jannchie/shields.rs/compare/v0.4.0...v0.5.0)

### :sparkles: Features

- **builder**: add logo and logo_color support to badge builders && update tests - By [Jianqi Pan](mailto:jannchie@gmail.com) in [36ba447](https://github.com/Jannchie/shields.rs/commit/36ba447)

### :memo: Documentation

- **readme**: add svg bitwise-identical note && list supported styles && add advanced usage example - By [Jianqi Pan](mailto:jannchie@gmail.com) in [963d8f6](https://github.com/Jannchie/shields.rs/commit/963d8f6)

### :wrench: Chores

- **deps**: update dependencies in lock file - By [Jianqi Pan](mailto:jannchie@gmail.com) in [6b892bf](https://github.com/Jannchie/shields.rs/commit/6b892bf)

## v0.4.0

[v0.3.0...v0.4.0](https://github.com/Jannchie/shields.rs/compare/v0.3.0...v0.4.0)

### :sparkles: Features

- **color-support**: add csscolorparser and improve color handling && enable more tests - By [Jianqi Pan](mailto:jannchie@gmail.com) in [089ef6f](https://github.com/Jannchie/shields.rs/commit/089ef6f)
- **logo-support**: add logo support to svg badges && update templates for logo rendering - By [Jianqi Pan](mailto:jannchie@gmail.com) in [f93a8b7](https://github.com/Jannchie/shields.rs/commit/f93a8b7)
- **templates**: add new svg badge templates && update Cargo.toml && modify gitignore - By [Jianqi Pan](mailto:jannchie@gmail.com) in [245bfe5](https://github.com/Jannchie/shields.rs/commit/245bfe5)

### :wrench: Chores

- **ci**: add github actions release workflow - By [Jianqi Pan](mailto:jannchie@gmail.com) in [ba398a8](https://github.com/Jannchie/shields.rs/commit/ba398a8)
- **tests**: update cache directory to target/tmp/cache - By [Jianqi Pan](mailto:jannchie@gmail.com) in [4191232](https://github.com/Jannchie/shields.rs/commit/4191232)

## v0.3.0

[v0.2.0...v0.3.0](https://github.com/Jannchie/shields.rs/compare/v0.2.0...v0.3.0)

### :rocket: Breaking Changes

- **badge**: add helvetica-bold font metrics && rename verdana to 11px-normal && add social badge rendering support - By [Jianqi Pan](mailto:jannchie@gmail.com) in [66b2f20](https://github.com/Jannchie/shields.rs/commit/66b2f20)
- **badge-builder**: create badge builder structs and traits && remove Badge struct - By [Jianqi Pan](mailto:jannchie@gmail.com) in [bdd77a9](https://github.com/Jannchie/shields.rs/commit/bdd77a9)

### :sparkles: Features

- **badge**: add link and extra_link options - By [Jianqi Pan](mailto:jannchie@gmail.com) in [4fb1a67](https://github.com/Jannchie/shields.rs/commit/4fb1a67)
- **badge**: support two links && increase color cache size && simplify code - By [Jianqi Pan](mailto:jannchie@gmail.com) in [77abad8](https://github.com/Jannchie/shields.rs/commit/77abad8)
- **badge-rendering**: capitalize label && handle links dynamically in templates - By [Jianqi Pan](mailto:jannchie@gmail.com) in [d0a8890](https://github.com/Jannchie/shields.rs/commit/d0a8890)
- **builder**: add support for multiple link states in badge builder && add tests for social and plastic badges with links - By [Jianqi Pan](mailto:jannchie@gmail.com) in [8a9bc5c](https://github.com/Jannchie/shields.rs/commit/8a9bc5c)
- **svg-badge**: add support for badge links && refactor badge parameters - By [Jianqi Pan](mailto:jannchie@gmail.com) in [565aa50](https://github.com/Jannchie/shields.rs/commit/565aa50)
- **svg-optimization**: minify svg templates && update gitignore && disable builder tests && adjust svg comparison test - By [Jianqi Pan](mailto:jannchie@gmail.com) in [54043c7](https://github.com/Jannchie/shields.rs/commit/54043c7)

### :art: Refactors

- **badge**: simplify plastic badge rendering - By [Jianqi Pan](mailto:jannchie@gmail.com) in [f5f5e74](https://github.com/Jannchie/shields.rs/commit/f5f5e74)
- **badge-rendering**: remove unnecessary has_message and has_link variables - By [Jianqi Pan](mailto:jannchie@gmail.com) in [b079d43](https://github.com/Jannchie/shields.rs/commit/b079d43)
- **build-script**: refactor and optimize minify logic - By [Jianqi Pan](mailto:jannchie@gmail.com) in [0aa1300](https://github.com/Jannchie/shields.rs/commit/0aa1300)
- **render-badge**: remove debug prints && improve code compactness - By [Jianqi Pan](mailto:jannchie@gmail.com) in [8091ebd](https://github.com/Jannchie/shields.rs/commit/8091ebd)
- **svg-rendering**: remove unused has_label field && consolidate test cases for badge generation - By [Jianqi Pan](mailto:jannchie@gmail.com) in [2d29f34](https://github.com/Jannchie/shields.rs/commit/2d29f34)
- **svg_compare**: remove normalize_svg function && update test_svg_compare to use raw svg - By [Jianqi Pan](mailto:jannchie@gmail.com) in [9759cb4](https://github.com/Jannchie/shields.rs/commit/9759cb4)

### :wrench: Chores

- **cargo**: update shields version && update description - By [Jianqi Pan](mailto:jannchie@gmail.com) in [56c9337](https://github.com/Jannchie/shields.rs/commit/56c9337)
- **dependencies**: remove unused dependencies - By [Jianqi Pan](mailto:jannchie@gmail.com) in [a22ba70](https://github.com/Jannchie/shields.rs/commit/a22ba70)

## v0.2.0

[edbd86b883fa61e799932cc9929104509c220b76...v0.2.0](https://github.com/Jannchie/shields.rs/compare/edbd86b883fa61e799932cc9929104509c220b76...v0.2.0)

### :sparkles: Features

- **color-util**: add lru cache to normalize_color and to_svg_color && refactor color parsing logic - By [Jianqi Pan](mailto:jannchie@gmail.com) in [bf3b67b](https://github.com/Jannchie/shields.rs/commit/bf3b67b)
- **color-util**: add color normalization and svg output support - By [Jianqi Pan](mailto:jannchie@gmail.com) in [fe2bb63](https://github.com/Jannchie/shields.rs/commit/fe2bb63)
- **svg-rendering**: add dynamic text color based on background && improve svg normalization - By [Jianqi Pan](mailto:jannchie@gmail.com) in [cff826c](https://github.com/Jannchie/shields.rs/commit/cff826c)
- **template**: add askama template for svg rendering - By [Jianqi Pan](mailto:jannchie@gmail.com) in [24a0474](https://github.com/Jannchie/shields.rs/commit/24a0474)
- **templates**: add svg templates && update gitignore && modify Cargo.toml - By [Jianqi Pan](mailto:jannchie@gmail.com) in [905c733](https://github.com/Jannchie/shields.rs/commit/905c733)

### :adhesive_bandage: Fixes

- **badge-rendering**: handle empty labels in different styles && add svg compare test cases - By [Jianqi Pan](mailto:jannchie@gmail.com) in [3d62a71](https://github.com/Jannchie/shields.rs/commit/3d62a71)

### :art: Refactors

- **badge-svg-template**: replace inline svg with askama template for flat-square and plastic badge - By [Jianqi Pan](mailto:jannchie@gmail.com) in [a0d5e85](https://github.com/Jannchie/shields.rs/commit/a0d5e85)
- **badge-template**: remove unused fields in svg context - By [Jianqi Pan](mailto:jannchie@gmail.com) in [5849423](https://github.com/Jannchie/shields.rs/commit/5849423)
- **cache**: refactor cache handling in preferred_width_of && adjust svg test sequence - By [Jianqi Pan](mailto:jannchie@gmail.com) in [45dfa0b](https://github.com/Jannchie/shields.rs/commit/45dfa0b)
- **readme-and-tests**: simplify usage example && optimize cache file naming - By [Jianqi Pan](mailto:jannchie@gmail.com) in [62cb3c5](https://github.com/Jannchie/shields.rs/commit/62cb3c5)
- **template**: rename template context struct && change svg template path - By [Jianqi Pan](mailto:jannchie@gmail.com) in [d9f6580](https://github.com/Jannchie/shields.rs/commit/d9f6580)

### :memo: Documentation

- **comments**: translate chinese comments to english && update test assertions to english - By [Jianqi Pan](mailto:jannchie@gmail.com) in [16d0d12](https://github.com/Jannchie/shields.rs/commit/16d0d12)
- **measurer**: translate comments and documentation to english - By [Jianqi Pan](mailto:jannchie@gmail.com) in [b97ab5b](https://github.com/Jannchie/shields.rs/commit/b97ab5b)
