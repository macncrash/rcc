# rcc - a Rust compiler written in C.
#   make                 build build/rcc with clang
#   make CC=.../ccc      build with ccc
#   make test-parse      run the parser acceptance test

CC ?= clang
ifeq ($(origin CC),default)
CC = clang
endif
CFLAGS ?= -std=c11 -Wall -Wextra -O1 -g
LDFLAGS ?=

SRCS := $(wildcard src/*.c)
BUILD := build
OBJS := $(patsubst src/%.c,$(BUILD)/obj/%.o,$(SRCS))

# Rebuild everything when the compiler or flags change (e.g. clang <-> ccc).
STAMP := $(BUILD)/obj/.cc-stamp
STAMP_TEXT := $(CC) $(CFLAGS) $(LDFLAGS)
ifneq ($(shell cat $(STAMP) 2>/dev/null),$(STAMP_TEXT))
$(shell mkdir -p $(BUILD)/obj && echo '$(STAMP_TEXT)' > $(STAMP))
endif

all: $(BUILD)/rcc

$(BUILD)/rcc: $(OBJS) $(STAMP)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJS)

# Header dependencies are tracked conservatively (every object depends on
# every header) because ccc may not support -MMD.
HDRS := $(wildcard src/*.h)
$(BUILD)/obj/%.o: src/%.c $(HDRS) $(STAMP) | $(BUILD)/obj
	$(CC) $(CFLAGS) -c -o $@ $<

$(BUILD)/obj:
	mkdir -p $@

test-parse: $(BUILD)/rcc
	tests/parse.sh

clean:
	rm -rf $(BUILD)/obj $(BUILD)/rcc

.PHONY: all clean test-parse
