    .section .text
    .globl __switch
__switch:
	# Allocate TaskContext on stack
    addi sp, sp, -13*8
    # save sp from first param
    sd sp, 0(a0)
    # save ra & s0-s11
    sd ra, 0*8(sp)
    sd s0, 1*8(sp)
    sd s1, 2*8(sp)
    sd s2, 3*8(sp)
    sd s3, 4*8(sp)
    sd s4, 5*8(sp)
    sd s5, 6*8(sp)
    sd s6, 7*8(sp)
    sd s7, 8*8(sp)
    sd s8, 9*8(sp)
    sd s9, 10*8(sp)
    sd s10, 11*8(sp)
    sd s11, 12*8(sp)
    # load sp from second param
    ld sp, 0(a1)
    # load ra & s0-s11
    ld ra, 0*8(sp)
    ld s0, 1*8(sp)
    ld s1, 2*8(sp)
    ld s2, 3*8(sp)
    ld s3, 4*8(sp)
    ld s4, 5*8(sp)
    ld s5, 6*8(sp)
    ld s6, 7*8(sp)
    ld s7, 8*8(sp)
    ld s8, 9*8(sp)
    ld s9, 10*8(sp)
    ld s10, 11*8(sp)
    ld s11, 12*8(sp)
    # Release TaskContext
    addi sp, sp, 13*8
    ret