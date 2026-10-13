.section .text.double_buffer_write,"ax",@progbits
	.globl	double_buffer_write
	.prefalign	4, .Lfunc_end6, nop
.type	double_buffer_write,@function
double_buffer_write:
	.cfi_startproc
	push rbp
	.cfi_def_cfa_offset 16
	push r15
	.cfi_def_cfa_offset 24
	push r14
	.cfi_def_cfa_offset 32
	push r13
	.cfi_def_cfa_offset 40
	push r12
	.cfi_def_cfa_offset 48
	push rbx
	.cfi_def_cfa_offset 56
	push rax
	.cfi_def_cfa_offset 64
	.cfi_offset rbx, -56
	.cfi_offset r12, -48
	.cfi_offset r13, -40
	.cfi_offset r14, -32
	.cfi_offset r15, -24
	.cfi_offset rbp, -16
	mov rbx, rsi
	mov r15, qword ptr [rdi]
	mov rax, qword ptr [r15 + 24]
	lea r12, [r15 + 16]
	mov r14, qword ptr [rip + membarrier::linux::STRATEGY@GOTPCREL]
	mov rbp, qword ptr [rip + <membarrier::linux::STRATEGY as core::ops::deref::Deref>::deref@GOTPCREL]
	jmp .LBB6_1
	.p2align	4
.LBB6_3:
	lock or	dword ptr [rsp - 64], 0
	mov rax, qword ptr [r15 + 24]
	cmp rax, r13
	je .LBB6_5
.LBB6_1:
	mov r13, rax
	mov qword ptr [r12], rax
	mov rdi, r14
	call rbp
	cmp byte ptr [rax], 2
	je .LBB6_3
	#MEMBARRIER
	mov rax, qword ptr [r15 + 24]
	cmp rax, r13
	jne .LBB6_1
.LBB6_5:
	mov qword ptr [rsp], r12
	and r13d, 1
	mov qword ptr [r15 + 8*r13], rbx
	mov rdi, rsp
	call qword ptr [rip + <<double_buffer_hb::Writer<_>>::write::IdleOnDrop as core::ops::drop::Drop>::drop@GOTPCREL]
	add rsp, 8
	.cfi_def_cfa_offset 56
	pop rbx
	.cfi_def_cfa_offset 48
	pop r12
	.cfi_def_cfa_offset 40
	pop r13
	.cfi_def_cfa_offset 32
	pop r14
	.cfi_def_cfa_offset 24
	pop r15
	.cfi_def_cfa_offset 16
	pop rbp
	.cfi_def_cfa_offset 8
	ret
