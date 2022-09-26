# Release notes
All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## Unreleased
- New CLI implementation #223 @mosuka

## 0.17.0 (2022-09-22)
- Make user dictionaries extensible #221 @mosuka

## 0.16.2 (2022-09-20)
- Support CC-CEDICT user dictionary #220 @mosuka
- Support ko-dic user dictionary #219 @mosuka
- Add feature for HTML reports #218 @mosuka
- Add benchmark for tokenize with word details #217 @mosuka
- Support UniDic user dictionary #215 @mosuka

## 0.16.1 (2022-09-13)
- Bump to 0.16.1 #214 @mosuka
- Use split_inclusive() to split text into sentences #213 @mosuka

## 0.16.0 (2022-09-11)
- Do not deserialize word details during tokenization #211 @mosuka

## 0.15.1 (2022-09-10)
- Use tokenize_str() in lindera-cli #210 @mosuka

## 0.15.0 (2022-09-09)
- Tokenize without word details in wakati mode #208 @mosuka
- Add bench for wakati #207 @mosuka
- Fix bench #200 @mosuka

## 0.14.0 (2022-07-02)
- Update CLI #198 @mosuka
- fix lindera Release page link #192 @mochi-sann

## 0.13.5 (2022-05-10)
- Include the ko-dic dictionary in the repository #190 @mosuka

## 0.13.4 (2022-05-09)
- Include the CC-CEDICT dictionary in the repository #189 @mosuka

## 0.13.3 (2022-05-08)
- Include the IPADIC dictionary in the repository #188 @mosuka

## 0.13.2 (2022-04-08)
- Remove unnecessary code #178 @mosuka

## 0.13.1 (2022-04-07)
- Refactoring #176 @mosuka

## 0.13.0 (2022-04-07)
- Some API has changed #175 @mosuka

## 0.12.6 (2022-04-06)
- Add derive #174 @mosuka

## 0.12.5 (2022-04-04)
- Add from_str function to Mode #173 @mosuka

## 0.12.4 (2022-04-04)
- Change UniDic download URL #172 @mosuka

## 0.12.3 (2022-04-04)
- Add UserDictionaryTypeError #171 @mosuka
- Add DictionaryTypeError #170 @mosuka

## 0.12.2 (2022-03-23)
- Bump to 0.12.2 #167 @mosuka
- Remove unused dependencies #166 @Kerollmops

## 0.12.1 (2022-03-22)
- Bump up to 0.12.1 #165 @mosuka
- Remove the Tokio dependency #164 @Kerollmops

## 0.12.0 (2022-03-16)
- Encapsulate lindera-core package behind the lindera package #157 @mosuka

## 0.11.1 (2022-03-08)
- Avoid building dictionaries not specified in features #156 @mosuka

## 0.11.0 (2022-03-07)
- Add feature flag for dictionaries #153 @mosuka
- Add dictionary to resources #149 @mosuka
- Update ci's Including branches setting #148 @abetomo
- Fix resources path #146 @mosuka

## 0.10.0 (2022-02-25)
- Bump up version to 0.10.0 #145 @mosuka
- Do not perform strict checks on left context id and right context id in unk.def #144 @mosuka
- Make tokenize method immutable #143 @ManyTheFish

## 0.9.1 (2022-02-24)
- Add feature flag for compressing dictionary #142 @mosuka

## 0.9.0 (2022-02-20)
- Compressing dictionaries by default #139 @mosuka
- Add version monitoring for github-action #130 @ikawaha
- Make it single binary #129 @mosuka
- Make the binary smaller by compressing the dictionary #126 @higumachan

## 0.8.1 (2021-11-13)
- Update yada requirement from 0.4 to 0.5 #124
- docs(readme): update the code example in readme #123 @abetomo
- chore: set the number of fields as a constant #122 @abetomo
- Make 3 methods to private methods #121 @johtani
- Add api comments #119 @johtani
- Add parameter to build_unk for variation of dictionaries #117 @jothani 
- Support binary user dictionary on CLI. #115 @mosuka
- Add Dockerfile. #113 @mosuka
- implement binary data reading and writing for user dictionary #114 @abetomo

## 0.8.0 (2021-08-22)
- Fix workflow. #112 @mosuka
- Bump up version to 0.8.0. #111 @mosuka
- Add DictionaryBuilder trait and some refactoring. #110 @mosuka
- Merge lindera-cli package. #108 @mosuka
- Add error struct #107 @mosuka
- do not download dictionary files when building 'docs.rs' #106 @abetomo
- Update dependencies. #104 @mosuka
- feat: implement a user dictionary to set costs #103 @abetomo
- chore: remove unnecessary variable assignments #102 @abetomo
- test: fix test warnings #101 @abetomo
- Download ipadic only when it doen't exist #99 @KitaitiMakoto

## 0.7.1 (2020-10-15)
- Bump up version to 0.7.1 #97 @mosuka
- Automate release tasks #96 @mosuka

## 0.7.0 (2020-10-12)
- Bump up version to 0.7.0 #94 @mosuka
- Move CLI to lindera-cli repository #92 @mosuka
- Upgrade Yada 0.4.0 #91 @johtani
- Fix backward_size reading #89 @johtani
- Update CI.yml #86 @mosuka
- Fix documentation bug. FST to DA #85 @johtani

## 0.6.0 (2020-10-07)
- Bump up version to 0.6.0 #82 @mosuka
- Update dependencies #81 @mosuka
- Add GitHub Actions integration and some refactoring #80 @mosuka
- Switch FST to Double Array #76 @johtani
- Add long-text benchmark #74 @johtani
- Update modules to 2018 #73 @johtani
- Use new method instead of default_normal #72 @johtani

## 0.5.1 (2020-07-06)
- CLI support for user dictionary. #67 @mocobeta

## 0.5.0 (2020-07-05)
- Support user dictionary. #64 @mocobeta

## 0.4.1 (2020-05-30)
- Bump up version #61 @mosuka
- Change download URL #60 @mosuka

## 0.4.0 (2020-05-22)
- Migrate to workspaces #57 @mosuka

## 0.3.5 (2020-04-30)
- Update dependencies #56 @mosuka

## 0.3.4 (2020-02-25)
- Change tokenizer constructor #55 @mosuka

## 0.3.3 (2020-02-25)
- Remove lifetime #54 @mosuka

## 0.3.2 (2020-02-20)
- Change word details to vec from strust #53 @mosuka

## 0.3.1 (2020-02-17)
- Update dependencies #52 @mosuka

## 0.3.0 (2020-02-14)
- Update dependencies #50 @mosuka

## 0.2.1 (2020-02-12)
- Bump up version #49 @mosuka
- Add formatter #48 @mosuka

## 0.2.0 (2020-02-10)
- Bump up version #46 @mosuka
- Delete unsed #45 @mosuka

## 0.1.5 (2020-02-08)
- Update lindera-ipadic #44 @mosuka
- Delete SystemDict #43 @mosuka

## 0.1.5 (2020-02-06)
- Add dictionary #40 @mosuka

## 0.1.4 (2020-02-05)
- Add dictionary builder #39 @mosuka
- Delete unnecessary dependencies #38 @mosuka

## 0.1.3 (2020-02-03)
- Split the package #35 @mosuka
- Splir the package #34 @mosuka
- Fix bugs #33 @mosuka
- Refactoring #32 @mosuka
- Fix typo #31 @mosuka
- Update docs #30 @mosuka
- Split the module #29 @mosuka
- Enrich word detail #28 @mosuka
- Support output in JSON format #26 @mosuka
- Make single command #25 @mosuka
- Enrich word details #23 @mosuka
- Support tokenization mode (normal and search) with Lindera CLI #22 @mosuka
- Fix build_fst #18 @mosuka
- Refactoring #16 @mosuka
- Rename project #14 @mosuka
- Update doc #12 @mosuka
- Add docs #11 @mosuka
- Add mokuzu command #10 @mosuka
- Fix typo #9 @ikawaha
- Fix unused imports and variables #6 @mosuka
- Restore the missing file #5 @mosuka
- Formatting Rust code #4 @mosuka
- Replace kuromoji to mokuzu #2 @johtani


## 0.0.0 (2020-01-22)
- Fork from @fulmicoton's project by @mosuka
