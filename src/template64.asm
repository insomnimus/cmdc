format PE64 console
entry start

include "win64wxp.inc"

MAX_CMD = 32767
PTR fix QWORD

section ".idata" import data readable writeable ;{
	library kernel32, "kernel32.dll", msvcrt, "msvcrt.dll", shlwapi, "shlwapi.dll"
	include "api/kernel32.inc"
	import msvcrt, printf, "wprintf_s", fprintf, "fwprintf_s", fdopen, "_wfdopen"
	import shlwapi, PathGetArgsW, "PathGetArgsW"
;}

section ".bss" data readable writeable ;{
	PINFO PROCESS_INFORMATION
	SINFO STARTUPINFO \
		sizeof.STARTUPINFO,\ ; cb
		0,\ ; lpReserved
		0,\ ; lpDesktop
		0,\ ; lpTitle,
		0,\ ; dwX
		0,\ ; dwY
		0,\ ; dwXSize
		0,\ ; dwYSize
		0,\ ; dwXCountChars
		0,\ ; dwYCountChars
		0,\ ; dwFillAttribute
		0,\ ; dwFlags
		0,\ ; wShowWindow
		0,\ ; cbReserved2
		0,\ ; lpReserved2
		0,\ ; hStdInput
		0,\ ; hStdOutput
		0 ; hStdError

	ERR_TOO_LONG du "error: failed to execute the command because the command string exceeds maximum allowed size", 10, 0
	ERROR_TEMPLATE du "error executing the command `%s`:", 10, "%s", 0
	LAST_ERROR dq 0
	EXIT_CODE dd 0
	COMMAND du "replaceme", MAX_CMD dup (0)
;}

section ".text" code readable executable ;{
	proc strlen uses rdi, string:PTR ;{
		mov rdi, rcx
		mov rcx, -1
		xor rax, rax
		cld
		repne scasw
		xor  rcx, -1
		dec rcx
		mov rax, rcx

		ret
	endp
	;}

	start: ;{
		sub rsp, 8 ; align the stack on 16-byte boundary
		fastcall strlen, COMMAND
		mov rbx, rax
		invoke PathGetArgsW, <invoke GetCommandLineW>
		mov rsi, rax
		fastcall strlen, rsi
		; Skip some ops if there're no args.
		cmp rax, 0
		je .exec_cmd

		add rax, rbx
		cmp rax, MAX_CMD - 1
		jae .cmd_too_long
		; Set the write index to the end of the COMMAND string.
		lea rdi, [COMMAND + (rbx * 2)] ; * 2 for unicode
		; append a space and the args to the command string.
		mov word [rdi], " "
		add rdi, 2
		sub rax, rbx ; subtract COMMAND.length because we added it previously
		mov rcx, rax ; counter (length of the argument string that we receive dynamically)
		cld ; clear direction flag
		rep movsw ; copy string

	.exec_cmd:
		invoke CreateProcessW,\
			NULL,\ ; module name
			COMMAND,\ ; command string
			NULL,\ ; process attr
			NULL,\ ; thread attr
			FALSE,\ ; inherit handles = false
			0,\ ; creation flags
			NULL,\ ; env block
			NULL,\ ; starting directory
			SINFO,\ ; startup info
			PINFO ; out process info

		; check return code
		cmp eax, FALSE
		je .err_exit

		invoke WaitForSingleObject, [PINFO.hProcess], -1
		invoke GetExitCodeProcess, [PINFO.hProcess], EXIT_CODE
		invoke CloseHandle, [PINFO.hProcess]
		invoke CloseHandle, [PINFO.hThread]

		invoke ExitProcess, [EXIT_CODE]
		jmp .return

	.err_exit:
		invoke GetLastError
		invoke FormatMessageW,\
			FORMAT_MESSAGE_FROM_SYSTEM or FORMAT_MESSAGE_ALLOCATE_BUFFER,\
			0,\
			rax,\
			LANG_NEUTRAL,\
			LAST_ERROR,\
			0, 0
		invoke fdopen, 2, "a"
		invoke fprintf, rax, ERROR_TEMPLATE, COMMAND, qword [LAST_ERROR]
		invoke ExitProcess, -2
		jmp .return

	.cmd_too_long:
	invoke fdopen, 2, "a"
		invoke fprintf, rax, ERR_TOO_LONG
		invoke ExitProcess, -1

	.return:
		add rsp, 8
		ret
	;}
;}
