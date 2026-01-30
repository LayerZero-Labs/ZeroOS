# Configuration passed from caller
LIB_FFI ?= libc_staticlib.a
CC ?= riscv64-unknown-elf-gcc
CFLAGS ?= -O2 -g -march=rv64imac -mabi=lp64 -ffreestanding -nostdlib -mcmodel=medany
LINKER ?= linker.ld
LIB_DIR ?= $(HOME)/.zeroos/musl/riscv64-linux-musl/lib
LDFLAGS ?= -T $(LINKER) -nostdlib -static -L $(LIB_DIR) -Wl,--gc-sections
# NOTE: For static linking, library order matters. libgcc can introduce new
# undefined symbols (e.g. pthread_* from unwind code). In practice, libc, libgcc
# and libpthread can form a cyclic dependency set, so we link them as a group.
LDLIBS ?= -Wl,--start-group -lc -lgcc -lpthread -Wl,--end-group
OUTPUT_DIR ?= $(PWD)/build
BIN := $(OUTPUT_DIR)/c-smoke
OBJ := $(OUTPUT_DIR)/main.o

# Targets
all: c-smoke

c-smoke: $(BIN)

$(BIN): $(OBJ) $(LIB_FFI)
	$(CC) $(CFLAGS) $(LDFLAGS) $(OBJ) $(LIB_FFI) $(LDLIBS) -o $(BIN)

$(OBJ): src/main.c | $(OUTPUT_DIR)
	$(CC) $(CFLAGS) -c src/main.c -o $(OBJ)

$(OUTPUT_DIR):
	mkdir -p $(OUTPUT_DIR)

clean:
	rm -f $(OBJ) $(BIN) src/*.o

run: c-smoke
	spike $(BIN)

.PHONY: all clean run c-smoke
