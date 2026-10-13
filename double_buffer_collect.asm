.section .text.double_buffer_collect,"ax",@progbits
	.globl	double_buffer_collect
	.prefalign	4, .Lfunc_end5, nop
.type	double_buffer_collect,@function
double_buffer_collect:
	.cfi_startproc
	push r15
	.cfi_def_cfa_offset 16
	push r14
	.cfi_def_cfa_offset 24
	push r13
	.cfi_def_cfa_offset 32
	push r12
	.cfi_def_cfa_offset 40
	push rbx
	.cfi_def_cfa_offset 48
	.cfi_offset rbx, -48
	.cfi_offset r12, -40
	.cfi_offset r13, -32
	.cfi_offset r14, -24
	.cfi_offset r15, -16
	mov rbx, rsi
	mov r15, qword ptr [rdi]
	mov r12, qword ptr [rdi + 8]
	cmp byte ptr [r12], 0
	jne .LBB5_11
	mov r13, qword ptr [r15 + 24]
	lea rax, [r13 + 1]
	xor ecx, ecx
	cmp r13, -3
	cmovne rcx, rax
	mov qword ptr [r15 + 24], rcx
	mov rdi, qword ptr [rip + membarrier::linux::STRATEGY@GOTPCREL]
	call qword ptr [rip + <membarrier::linux::STRATEGY as core::ops::deref::Deref>::deref@GOTPCREL]
	movzx eax, byte ptr [rax]
	test eax, eax
	je .LBB5_9
	cmp eax, 1
	jne .LBB5_8
	mov rdi, qword ptr [rip + membarrier::linux::mprotect::BARRIER@GOTPCREL]
	call qword ptr [rip + <membarrier::linux::mprotect::BARRIER as core::ops::deref::Deref>::deref@GOTPCREL]
	mov r14, rax
	mov rdi, rax
	call qword ptr [rip + pthread_mutex_lock@GOTPCREL]
	test eax, eax
	jne .LBB5_7
	mov rdi, qword ptr [r14 + 40]
	mov rsi, qword ptr [r14 + 48]
	mov edx, 3
	call qword ptr [rip + mprotect@GOTPCREL]
	test eax, eax
	jne .LBB5_7
	mov rax, qword ptr [r14 + 40]
	lock inc	qword ptr [rax]
	mov rdi, qword ptr [r14 + 40]
	mov rsi, qword ptr [r14 + 48]
	xor edx, edx
	call qword ptr [rip + mprotect@GOTPCREL]
	test eax, eax
	jne .LBB5_7
	mov rdi, r14
	call qword ptr [rip + pthread_mutex_unlock@GOTPCREL]
	test eax, eax
	je .LBB5_10
	jmp .LBB5_7
.LBB5_9:
	mov edi, 324
	mov esi, 8
	xor edx, edx
	xor eax, eax
	call qword ptr [rip + syscall@GOTPCREL]
	test rax, rax
	jns .LBB5_10
.LBB5_7:
	call qword ptr [rip + abort@GOTPCREL]
.LBB5_8:
	lock or	dword ptr [rsp - 64], 0
.LBB5_10:
	mov qword ptr [r12], 1
	mov qword ptr [r12 + 8], r13
.LBB5_11:
	mov eax, 1
	mov r14, qword ptr [rip + std::thread::functions::yield_now@GOTPCREL]
	jmp .LBB5_12
	.p2align	4
.LBB5_14:
	call r14
	mov rax, qword ptr [r12]
.LBB5_12:
	cmp rax, 1
	jne .LBB5_14
	mov rcx, qword ptr [r12 + 8]
	mov rax, qword ptr [r15 + 16]
	cmp rax, rcx
	je .LBB5_14
	and ecx, 1
	mov rax, qword ptr [r15 + 8*rcx]
	mov qword ptr [r15 + 8*rcx], rbx
	mov qword ptr [r12], 0
	pop rbx
	.cfi_def_cfa_offset 40
	pop r12
	.cfi_def_cfa_offset 32
	pop r13
	.cfi_def_cfa_offset 24
	pop r14
	.cfi_def_cfa_offset 16
	pop r15
	.cfi_def_cfa_offset 8
	ret
