#include "classic.h"
#include "keyboard.h"
#include "io/io.h"
#include <stdint.h>
#include "kernel.h"
#include "idt/idt.h"
#include "task/task.h"

#define SHIFT_LEFT 0x2A
#define SHIFT_RIGHT 0x36
#define CTRL 0x1D


int classic_keyboard_init();
void classic_keyboard_handle_interrupt();

// Scancode -> ASCII
static const uint8_t keyboard_scan_set_one[] = {
    0x00,  ESC,  '1',  '2',     /* 0x00 */
     '3',  '4',  '5',  '6',     /* 0x04 */
     '7',  '8',  '9',  '0',     /* 0x08 */
     '-',  '=',   BS, '\t',     /* 0x0C */
     'q',  'w',  'e',  'r',     /* 0x10 */
     't',  'y',  'u',  'i',     /* 0x14 */
     'o',  'p',  '[',  ']',     /* 0x18 */
    ENTER, 0x00,  'a',  's',     /* 0x1C */
     'd',  'f',  'g',  'h',     /* 0x20 */
     'j',  'k',  'l',  ';',     /* 0x24 */
    '\'',  '`', 0x00, '\\',     /* 0x28 */
     'z',  'x',  'c',  'v',     /* 0x2C */
     'b',  'n',  'm',  ',',     /* 0x30 */
     '.',  '/', 0x00,  '*',     /* 0x34 */
    0x00,  ' ', 0x00, 0x00,     /* 0x38 */
    0x00, 0x00, 0x00, 0x00,     /* 0x3C */
    0x00, 0x00, 0x00, 0x00,     /* 0x40 */
    0x00, 0x00, 0x00,  '7',     /* 0x44 */
     '8',  '9',  '-',  '4',     /* 0x48 */
     '5',  '6',  '+',  '1',     /* 0x4C */
     '2',  '3',  '0',  '.',     /* 0x50 */
    0x00, 0x00, 0x00, 0x00,     /* 0x54 */
    0x00, 0x00, 0x00, 0x00      /* 0x58 */
};

// Scancode -> ASCII
static const uint8_t keyboard_scan_set_two[] = {
    0x00,  ESC,  '!',  '@',     /* 0x00 */
     '#',  '$',  '%',  '^',     /* 0x04 */
     '&',  '*',  '(',  ')',     /* 0x08 */
     '_',  '+',   BS, '\t',     /* 0x0C */
     'Q',  'W',  'E',  'R',     /* 0x10 */
     'T',  'Y',  'U',  'I',     /* 0x14 */
     'O',  'P',  '{',  '}',     /* 0x18 */
    ENTER, 0x00,  'A',  'S',     /* 0x1C */
     'D',  'F',  'G',  'H',     /* 0x20 */
     'J',  'K',  'L',  ':',     /* 0x24 */
     '"',  '~', 0x00,  '|',     /* 0x28 */
     'Z',  'X',  'C',  'V',     /* 0x2C */
     'B',  'N',  'M',  '<',     /* 0x30 */
     '>',  '?', 0x00,  '*',     /* 0x34 */
    0x00,  ' ', 0x00, 0x00,     /* 0x38 */
    0x00, 0x00, 0x00, 0x00,     /* 0x3C */
    0x00, 0x00, 0x00, 0x00,     /* 0x40 */
    0x00, 0x00, 0x00,  '7',     /* 0x44 */
     '8',  '9',  '-',  '4',     /* 0x48 */
     '5',  '6',  '+',  '1',     /* 0x4C */
     '2',  '3',  '0',  '.',     /* 0x50 */
    0x00, 0x00, 0x00, 0x00,     /* 0x54 */
    0x00, 0x00, 0x00, 0x00      /* 0x58 */
};

static struct keyboard classic_keyboard = {
    .init = classic_keyboard_init,
    .name = "classic",
    .shift = 0,
    .ctrl = 0,
};

int classic_keyboard_init() {
    idt_register_interrupt_callback(KEYBOARD_INTERRUPT, classic_keyboard_handle_interrupt);
    outb(PS2_PORT, PS2_COMMAND_ENABLE_FIRST_PORT);
    return 0;
}

static uint8_t classic_keyboard_scancode_to_char(uint8_t scancode) {
    size_t size_get_one = sizeof(keyboard_scan_set_one) / sizeof(uint8_t);
    if (scancode > size_get_one) {
        return 0;
    }

    uint8_t* codes;
    if (classic_keyboard.shift) {
        codes = (uint8_t*)keyboard_scan_set_two;
    }
    else {
        codes = (uint8_t*)keyboard_scan_set_one;
    }

    char c = codes[scancode];
    return c;
}

void classic_keyboard_handle_interrupt() {
    uint8_t scancode = 0;
    scancode = insb(KEYBOARD_INPUT_PORT);
    insb(KEYBOARD_INPUT_PORT);

    if (scancode & CLASSIC_KEYBOARD_KEY_RELEASED) {
        if (scancode == SHIFT_LEFT) {
            classic_keyboard.shift &= 0x02;
        }
        else if (scancode == SHIFT_RIGHT) {
            classic_keyboard.shift &= 0x01;
        }
        else if (scancode == CTRL) {
            classic_keyboard.ctrl = 0;
        }
        return;
    }
    if (scancode == SHIFT_LEFT) {
        classic_keyboard.shift |= 0x01;
        return;
    }
    else if (scancode == SHIFT_RIGHT) {
        classic_keyboard.shift |= 0x02;
        return;
    }
    else if (scancode == CTRL) {
        classic_keyboard.ctrl = 1;
        return;
    }
    
    uint8_t c = classic_keyboard_scancode_to_char(scancode);
    if (c != 0) {
        keyboard_push(c);
    }
}

struct keyboard *classic_init() {
    return &classic_keyboard;
}
    