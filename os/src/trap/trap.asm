
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 4
__alltraps:
	# move to kernel stack
    csrrw sp, sscratch, sp
	# allocate TrapContext
	addi sp, sp, -39*8
	# save x1 & x3~x31
	sd x1, 1*8(sp)
	# x2(sp) will be saved later
	sd x3, 3*8(sp)
	sd x4, 4*8(sp)
	sd x5, 5*8(sp)
	sd x6, 6*8(sp)
	sd x7, 7*8(sp)
	sd x8, 8*8(sp)
	sd x9, 9*8(sp)
	sd x10, 10*8(sp)
	sd x11, 11*8(sp)
	sd x12, 12*8(sp)
	sd x13, 13*8(sp)
	sd x14, 14*8(sp)
	sd x15, 15*8(sp)
	sd x16, 16*8(sp)
	sd x17, 17*8(sp)
	sd x18, 18*8(sp)
	sd x19, 19*8(sp)
	sd x20, 20*8(sp)
	sd x21, 21*8(sp)
	sd x22, 22*8(sp)
	sd x23, 23*8(sp)
	sd x24, 24*8(sp)
	sd x25, 25*8(sp)
	sd x26, 26*8(sp)
	sd x27, 27*8(sp)
	sd x28, 28*8(sp)
	sd x29, 29*8(sp)
	sd x30, 30*8(sp)
	sd x31, 31*8(sp)
	# save sstatus
    csrr t0, sstatus
    sd t0, 32*8(sp)
	# save sepc
    csrr t1, sepc
    sd t1, 33*8(sp)
	# save sp from sscratch
    csrr t2, sscratch
    sd t2, 2*8(sp)
	# sp now point to TrapContext
	# load kernel satp
	ld t0, 36*8(sp)
	# load kernel space TrapContext address
	ld sp, 38*8(sp)
	# switch to kernel space
	csrw satp, t0
	sfence.vma
	# load trap_handler address
    ld x6, 34*8(sp)
	# call trap_handler with TrapContext as param
    mv a0, sp
	jalr x6
__restore:
	# moved to new kernel stack
	# the new kernel stack is either return value of trap_handler
	# where the kernel stack do not change
	# or manually called by launch to spawn new process
	# load user satp
	ld t0, 35*8(sp)
	# load user space TrapContext address
	ld sp, 37*8(sp)
	# switch to user space
	csrw satp, t0
	sfence.vma
	# load sstatus
	ld t0, 32*8(sp)
	csrw sstatus, t0
	# load sepc
	ld t1, 33*8(sp)
	csrw sepc, t1
	# load user stack sp(x2) to sscratch
	ld t2, 2*8(sp)
	csrw sscratch, t2
	# save x1 & x3~x31
	ld x1, 1*8(sp)
	# x2(sp) will be loaded later
	ld x3, 3*8(sp)
	ld x4, 4*8(sp)
	ld x5, 5*8(sp)
	ld x6, 6*8(sp)
	ld x7, 7*8(sp)
	ld x8, 8*8(sp)
	ld x9, 9*8(sp)
	ld x10, 10*8(sp)
	ld x11, 11*8(sp)
	ld x12, 12*8(sp)
	ld x13, 13*8(sp)
	ld x14, 14*8(sp)
	ld x15, 15*8(sp)
	ld x16, 16*8(sp)
	ld x17, 17*8(sp)
	ld x18, 18*8(sp)
	ld x19, 19*8(sp)
	ld x20, 20*8(sp)
	ld x21, 21*8(sp)
	ld x22, 22*8(sp)
	ld x23, 23*8(sp)
	ld x24, 24*8(sp)
	ld x25, 25*8(sp)
	ld x26, 26*8(sp)
	ld x27, 27*8(sp)
	ld x28, 28*8(sp)
	ld x29, 29*8(sp)
	ld x30, 30*8(sp)
	ld x31, 31*8(sp)
	# release TrapContext
	addi sp, sp, 39*8
	# switch user/kernel stack
	csrrw sp, sscratch, sp
    sret
    .align 4
__ktraps:
	j trap_from_kernel