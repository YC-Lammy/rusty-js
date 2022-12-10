	.text
	.def	 @feat.00;
	.scl	3;
	.type	0;
	.endef
	.globl	@feat.00
.set @feat.00, 0
	.file	"intpr.ll"
	.def	 run_codes;
	.scl	2;
	.type	32;
	.endef
	.globl	run_codes                       # -- Begin function run_codes
	.p2align	4, 0x90
run_codes:                              # @run_codes
.seh_proc run_codes
# %bb.0:
	subq	$40, %rsp
	.seh_stackalloc 40
	.seh_endprologue
	.p2align	4, 0x90
.LBB0_1:                                # %LoopStart
                                        # =>This Loop Header: Depth=1
                                        #     Child Loop BB0_2 Depth 2
	movq	$0, (%rsp)
	.p2align	4, 0x90
.LBB0_2:                                # %LoopStart
                                        #   Parent Loop BB0_1 Depth=1
                                        # =>  This Inner Loop Header: Depth=2
	movq	(%rsp), %rax
	leaq	1(%rax), %r9
	cmpq	%rdx, %rax
	movq	(%rcx,%rax,8), %rax
	movq	%rax, 8(%rsp)
	movq	%r9, (%rsp)
	jne	.LBB0_7
# %bb.3:                                # %RunCode
                                        #   in Loop: Header=BB0_2 Depth=2
	movq	8(%rsp), %rax
	cmpq	$4, %rax
	jb	.LBB0_2
# %bb.4:                                # %RunCode
                                        #   in Loop: Header=BB0_2 Depth=2
	je	.LBB0_1
# %bb.5:                                # %RunCode
                                        #   in Loop: Header=BB0_2 Depth=2
	cmpq	$5, %rax
	je	.LBB0_2
# %bb.6:                                # %IfUnknownCode
	movb	$1, 8(%r8)
	movq	$0, (%r8)
.LBB0_7:                                # %IfEnded
	addq	$40, %rsp
	retq
	.seh_endproc
                                        # -- End function
