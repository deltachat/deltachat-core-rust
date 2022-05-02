{
  # documentation about the format of this file can be found under https://gyp.gsrc.io/docs/InputFormatReference.md
  # Variables can be specified when calling node-gyp as so:
  #   node-gyp configure -- -Dvarname=value
  "variables": {
    # Whether to use a system-wide installation of deltachat-core
    # using pkg-config.  Set to either "true" or "false".
    "USE_SYSTEM_LIBDELTACHAT%": "<!(echo $USE_SYSTEM_LIBDELTACHAT)",
  },
  "targets": [
    {
      "target_name": "deltachat",
      "sources": ["./src/module.c"],
      "include_dirs": ["<!(node -e \"require('napi-macros')\")"],
      "conditions": [
        [
          "OS == 'win'",
          {
            "include_dirs": ["../deltachat-ffi"],
            "libraries": [
              "../../target/release/deltachat.dll.lib",
            ],
            "conditions": [
              [
                "USE_SYSTEM_LIBDELTACHAT == 'true'",
                {
                  "cflags": ["<!(pkg-config --cflags deltachat)"],
                  "libraries": ["<!(pkg-config --libs deltachat)"],
                },
              ],
            ],
          },
        ],
        [
          "OS == 'linux' or OS == 'mac'",
          {
            "libraries": ["-lpthread"],
            "cflags": ["-std=gnu99"],
            "conditions": [
              [
                "USE_SYSTEM_LIBDELTACHAT != 'true'",
                {
                  "include_dirs": ["../deltachat-ffi"],
                  "ldflags": ["-Wl,-Bsymbolic"], # Prevent sqlite3 from electron from overriding sqlcipher
                  "libraries": [
                    "../../target/release/libdeltachat.a",
                    "-ldl",
                  ],
                  "conditions": [],
                },
                {
                  # USE_SYSTEM_LIBDELTACHAT == 'true'
                  "cflags": ["<!(pkg-config --cflags deltachat)"],
                  "libraries": ["<!(pkg-config --libs deltachat)"],
                },
              ],
              [
                "OS == 'mac'",
                {
                  "libraries": [
                    "-framework CoreFoundation",
                    "-framework CoreServices",
                    "-framework Security",
                    "-lresolv",
                  ],
                },
                {
                  # OS == 'linux'
                  "libraries": ["-lm", "-lrt"],
                },
              ],
            ],
          },
        ],
      ],
    },
  ],
}
