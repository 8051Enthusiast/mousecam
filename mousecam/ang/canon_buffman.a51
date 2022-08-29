; lookup a value in the huffman table and
; write it if the current byte is full
COMP:
	MOV A, R7		;1
	MOV DPTR, #VAL_TAB	;3
	MOVC A, @A+DPTR		;5
	XCH A, R7		;2
	MOV DPTR, #LEN_TAB	;3
	MOVC A, @A+DPTR		;5
	MOV R6, A		;1
	ADD A, #0F8h		;2
	JNC TAIL		;3 -- 25
	JZ EIGHT_LEFT		;3
	MOV R6, A		;1
	MOV A, R5		;1
INSERT_ONES:			;| per bit
	SETB C			;|1
	RLC A			;|1
	JNC SKIP_SEND		;|3
				;|| per byte
	MOV R5, A		;||1
	ACALL SEND		;||4 + 29
	MOV A, R5		;||1 -- 35
SKIP_SEND:
	DJNZ R6, INSERT_ONES	;|4 -- 9
	MOV R5, A		;1
; insert 8 bits as-is into the bitstream 
EIGHT_LEFT:
	MOV R6, #8		;2
TAIL:
	MOV A, R7		;1
SHIFT_LOOP:			;| per bit
	RLC A			;|1
	XCH A, R5		;|2
	RLC A			;|1
	XCH A, R5		;|2
	JNC SKIP_SEND2		;|3
				;|| per byte
	MOV R7, A		;||1
	ACALL SEND		;||4 + 29
	MOV A, R7		;||1 -- 35
SKIP_SEND2:
	DJNZ R6, SHIFT_LOOP	;|4 -- 13
	RET			;4
; do the neccessary IO to send the byte
SEND:				;|| per byte
	MOV 0B7h, #0FFh		;||3
	MOV A, 0F1h		;||2
	ANL A, #02h		;||2
	JNB ACC.1, SEND		;||5
	ANL 0F1h, #0FCh		;||3
	MOV A, 0F2h		;||2
	CJNE A, #064h, SEND	;||4
	MOV 0F3h, R5		;||2
	MOV R5, #1		;||2
	RET			;||4 -- 29

; note that this is mostly identical to the first
; half, except for the branch to ALTTEST_IE1 which
; makes sure to set 0xee
ALTCOMP:
	MOV A, R7		;1
	MOV DPTR, #VAL_TAB	;3
	MOVC A, @A+DPTR		;5
	XCH A, R7		;2
	MOV DPTR, #LEN_TAB	;3
	MOVC A, @A+DPTR		;5
	MOV R6, A		;1
	ADD A, #0F8h		;2
	JNC ALTTAIL		;3 -- 25
	JZ ALTEIGHT_LEFT	;3
	MOV R6, A		;1
	MOV A, R5		;1
ALTINSERT_ONES:			;| per bit
	SETB C			;|1
	RLC A			;|1
	JNC ALTSKIP_SEND	;|3
				;|| per byte
	MOV R5, A		;||1
	ACALL ALTSEND		;||4 + 29
	MOV A, R5		;||1 -- 35
ALTSKIP_SEND:
	DJNZ R6, ALTINSERT_ONES	;|4 -- 9
	MOV R5, A		;1
ALTEIGHT_LEFT:
	MOV R6, #8		;2
ALTTAIL:
	MOV A, R7		;1
ALTSHIFT_LOOP:			;| per bit
	RLC A			;|1
	XCH A, R5		;|2
	RLC A			;|1
	XCH A, R5		;|2
	JNC ALTSKIP_SEND2	;|3
				;|| per byte
	MOV R7, A		;||1
	ACALL ALTSEND		;||4 + 29
	MOV A, R7		;||1 -- 35
ALTSKIP_SEND2:
	DJNZ R6, ALTSHIFT_LOOP	;|4 -- 13
	RET			;4
ALTSEND:			;|| per byte
	MOV 0B7h, #0FFh		;||3
	MOV A, 0F1h		;||2
	ANL A, #02h		;||2
	JNB ACC.1, ALTTEST_IE1	;||5
	ANL 0F1h, #0FCh		;||3
	MOV A, 0F2h		;||2
	CJNE A, #064h, ALTSEND	;||4
	MOV 0F3h, R5		;||2
	MOV R5, #1		;||2
	RET			;||4 -- 29
ALTTEST_IE1:
	JNB IE1, ALTSEND	; note: IE1 is not unset for control flow
	ORL 0EEh, #080h
	AJMP SEND
