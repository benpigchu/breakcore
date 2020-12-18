
    .section .text
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp
	call trap_handler
__restore:
	# move to new kernel stack
    mv sp, a0
	# load epc from context
	ld t0, 1*8(sp)
	csrw sepc, t0
	# load user stack from context
	ld t1, 0*8(sp)
	csrw sscratch, t1
	# release TrapContext
	addi sp, sp, 2*8
	# switch user/kernel stack
	csrrw sp, sscratch, sp
    sret
