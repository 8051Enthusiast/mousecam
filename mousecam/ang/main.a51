	ORG 08000h
	; buffer for previous line (0x1e bytes)
	LINE_BUF	EQU	010h
	; buffer of rest of last frame while new frame is
	; being written (0xd2 bytes)
	PAUSE_BUF	EQU	02eh
	; base pointer in frame buffer where memory that is copied into
	; pause buffer is read
	PAUSE_BUF_FROM	CODE	(0C006h + 0384h - PAUSE_BUF - 0d2h)
	LINE_STD	EQU	7

; calls LOC without adding to the pause buffer
CALL_SEND MACRO LOC
	PUSH DPH		;3
	PUSH DPL		;3
	ACALL LOC		;4
	POP DPL			;2
	POP DPH			;2 -- 14
ENDM
; copies into the pause buffer and calls LOC to send it
MEM_PAUSE MACRO LOC
	PUSH DPH		 ;3
	PUSH DPL		 ;3
	MOV DPTR, #PAUSE_BUF_FROM;3
	MOV A, R1		 ;1
	MOVC A, @A+DPTR		 ;5
	MOV @R1, A		 ;2
	INC R1			 ;2
	ACALL LOC		 ;4
	POP DPL			 ;2
	POP DPH			 ;2 -- 27

ENDM
	; get the first frame ready
	MOV R5, #1
	CLR IE1
WAIT_ACT:
	JNB IE1, WAIT_ACT
	ORL 0EEh, #080h
	CLR IE1
WAIT_INACT:
	JNB IE1, WAIT_INACT
	ANL 0EEh, #07Fh
	CLR IE1
	ANL 0F1h, #0FCh
NEW_FRAME:
	; write the shutter time
	; uncompressed to the stream
	MOV 0B3h, #0
	MOV R0, 0B4h
	MOV R7, 0B4h
	ACALL EIGHT_LEFT
	MOV A, R0
	MOV R7, A
	ACALL EIGHT_LEFT
; ----------------------------
; |        Scanline 1        |
; ----------------------------
; during the first scanline, the first pixel
; is specially handled and all the other pixels
; are send as differnce to just one pixel and not
; two as is the case for the majority of pixels
	ANL 0EEh, #07Fh		;3
	MOV DPTR, #0C006h	;3
	MOV R0, #LINE_BUF	;2
	MOV R1, #PAUSE_BUF	;2
	MOV R3, #(LINE_STD-1)	;2
	CLR A			;1
	MOVC A, @A+DPTR		;5
	MOV @R0, A		;2
	INC R0			;2
	MOV R2, A		;1
	MOV R7, A		;1
	INC DPTR		;1

	; the first pixel of a frame
	; is sent uncompressed
	MEM_PAUSE EIGHT_LEFT	;27 -- 52
FL_SEC:
	CLR A			;1
	MOVC A, @A+DPTR		;5
	MOV @R0, A		;2
	INC R0			;2
	XCH A, R2		;2
	CLR C			;1
	SUBB A, R2		;1
	MOV R7, A		;1
	INC DPTR		;1
	MEM_PAUSE COMP		;27
	CJNE R0, #(LINE_BUF + 01Eh), FL_SEC	;4 -- 47
; ----------------------------
; |      Scanline 2-7        |
; ----------------------------
; during the first 7 scanelines, an extra pixel
; from the last 7 scanlines is stored in the mem buffer
SL_FST:
	; the first pixel of a given scanline has
	; to be specially handled since it is
	; only a difference to the first pixel
	; of the previous scanline and not of two
	CLR A			;1
	MOV R0, #LINE_BUF	;2
	MOVC A, @A+DPTR		;5
	MOV R2, A		;1
	XCH A, @R0		;3
	CLR C			;1
	SUBB A, @R0		;2
	INC R0			;2
	MOV R7, A		;1
	INC DPTR		;1
	MEM_PAUSE COMP		;27 -- 46 (+4)
SL_SEC:
	CLR A			;1
	MOVC A, @A+DPTR		;5
	XCH A, R2		;2
	ADD A, @R0		;2
	RRC A			;1
	CLR C			;1
	SUBB A, R2		;2
	MOV R7, A		;1
	MOV A, R2		;1
	MOV @R0, A		;2
	INC R0			;2
	INC DPTR		;1
	MEM_PAUSE COMP		;27
	CJNE R0, #(LINE_BUF + 01Eh), SL_SEC	;4 -- 52
	DJNZ R3, SL_FST		;4
; ----------------------------
; |      Scanline 8-17       |
; ----------------------------
	MOV R3, #(01Eh-2*LINE_STD)
TL_FST:
	CLR A			;1
	MOV R0, #LINE_BUF	;2
	MOVC A, @A+DPTR		;5
	MOV R2, A		;1
	XCH A, @R0		;3
	CLR C			;1
	SUBB A, @R0		;2
	INC R0			;2
	MOV R7, A		;1
	INC DPTR		;1
	CALL_SEND COMP		;14 -- 33 (+4)
TL_SEC:
	CLR A			;1
	MOVC A, @A+DPTR		;5
	XCH A, R2		;2
	ADD A, @R0		;2
	RRC A			;1
	CLR C			;1
	SUBB A, R2		;2
	MOV R7, A		;1
	MOV A, R2		;1
	MOV @R0, A		;2
	INC R0			;2
	INC DPTR		;1
	CALL_SEND COMP		;14
	CJNE R0, #(LINE_BUF + 01Eh), TL_SEC	;4 -- 39
	DJNZ R3, TL_FST		;4
; ----------------------------
; | Scanline 18-1E nonactive |
; ----------------------------
; the last 7 scanlines are split into two sections
; (to reduce branches), one for when the frame fetcher
; is active and one for when not because we need to
; regularly check the interrupt flag to make sure
; it is shut down after writing a whole frame
;
; also, because it is active, we cannot rely on the
; framebuffer itself to provide the pixels but instead
; use the buffer we built in lines 1 - 7
	MOV R3, #LINE_STD	;2
	CLR IE1			;3
	MOV R1, #PAUSE_BUF	;2
LLN_FST:
	MOV R0, #LINE_BUF	;2
	MOV A, @R1		;2
	MOV R2, A		;1
	XCH A, @R0		;3
	CLR C			;1
	SUBB A, @R0		;2
	INC R0			;2
	INC R1			;2
	MOV R7, A		;1
	JBC IE1, LLNFSKIP	;5
	ACALL ALTCOMP		;4 -- 34
LLN_SEC:
	MOV A, @R1		;2
	XCH A, R2		;2
	ADD A, @R0		;2
	RRC A			;1
	CLR C			;1
	SUBB A, R2		;1
	MOV R7, A		;1
	MOV A, R2		;1
	MOV @R0, A		;2
	INC R0			;2
	INC R1			;2
	JBC IE1, LLNSSKIP	;5
	ACALL ALTCOMP		;4
	CJNE R0, #(LINE_BUF + 1Eh), LLN_SEC	;4 -- 30
	DJNZ R3, LLN_FST	;4
	AJMP NEW_FRAME
LLNFSKIP:
	ORL 0EEh, #080h		;3
	SJMP LLA_FCONT		;3
LLNSSKIP:
	ORL 0EEh, #080h		;3
	SJMP LLA_SCONT		;3

; ----------------------------
; |   Scanline 18-1E active  |
; ----------------------------
LLA_FST:
	MOV R0, #LINE_BUF	;2
	MOV A, @R1		;2
	MOV R2, A		;1
	XCH A, @R0		;3
	CLR C			;1
	SUBB A, @R0		;2
	INC R0			;2
	INC R1			;2
	MOV R7, A		;1
	JBC IE1, LLAFSKIP	;5 (+6)
LLA_FCONT:
	ACALL COMP		;4 -- 35
LLA_SEC:
	MOV A, @R1		;2
	XCH A, R2		;2
	ADD A, @R0		;2
	RRC A			;1
	CLR C			;1
	SUBB A, R2		;1
	MOV R7, A		;1
	MOV A, R2		;1
	MOV @R0, A		;2
	INC R0			;2
	INC R1			;2
	JBC IE1, LLASSKIP	;5
LLA_SCONT:
	ACALL COMP		;4
	CJNE R0, #(LINE_BUF + 1Eh), LLA_SEC	;4 - 36
	DJNZ R3, LLA_FST
	AJMP NEW_FRAME
LLAFSKIP:
	ANL 0EEh, #07Fh		;3
	SJMP LLA_FCONT		;3
LLASSKIP:
	ANL 0EEh, #07Fh		;3
	SJMP LLA_SCONT		;3
$INCLUDE(canon_buffman.a51)
$INCLUDE(huff_table.a51)
	END
