format PE console
entry start

include "win32wxp.inc"

MAX_CMD = 32767
PTR fix DWORD

section ".idata" import data readable writeable ;{
	library kernel32, "kernel32.dll", msvcrt, "msvcrt.dll", shlwapi, "shlwapi.dll"
	include "api/kernel32.inc"
	import msvcrt, printf, "wprintf_s", fprintf, "fwprintf_s", fdopen, "_wfdopen"
	import shlwapi, PathGetArgsW, "PathGetArgsW"
;}

section ".data" data readable writeable ;{
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
	LAST_ERROR dd ?
	EXIT_CODE dd 0

	ANCHOR db "---CMDC COMMAND STRING---"
	COMMAND du "replaceme", MAX_CMD dup (0)
;}

section ".text" code readable executable ;{
	proc strlen stdcall uses edi, string:PTR ;{
		mov edi, dword [string]
		mov ecx, -1
		xor eax, eax
		cld
		repne scasw
		xor  ecx, -1
		dec ecx
		mov eax, ecx

		ret
	endp
	;}

	start: ;{
		stdcall strlen, COMMAND
		mov ebx, eax
		invoke PathGetArgsW, <invoke GetCommandLineW>
		mov esi, eax
		stdcall strlen, esi
		; Skip some ops if there're no args.
		cmp eax, 0
		je .exec_cmd

		add eax, ebx
		cmp eax, MAX_CMD - 1
		jae .cmd_too_long
		; Set the write index to the end of the COMMAND string.
		lea edi, [COMMAND + (ebx * 2)] ; * 2 for unicode
		; append a space and the args to the command string.
		mov word [edi], " "
		add edi, 2
		sub eax, ebx ; subtract COMMAND.length because we added it previously
		mov ecx, eax ; counter (length of the argument string that we receive dynamically)
		cld ; clear direction flag
		rep movsw ; copy string

	.exec_cmd:
		invoke CreateProcessW,\
			NULL,\ ; module name
			COMMAND,\ ; command string
			NULL,\ ; process attr
			NULL,\ ; thread attr
			TRUE,\ ; inherit handles = false
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
			eax,\
			LANG_NEUTRAL,\
			LAST_ERROR,\
			0, 0
		invoke fdopen, 2, "a"
		invoke fprintf, eax, ERROR_TEMPLATE, COMMAND, dword [LAST_ERROR]
		invoke ExitProcess, -2
		jmp .return

	.cmd_too_long:
	invoke fdopen, 2, "a"
		invoke fprintf, eax, ERR_TOO_LONG
		invoke ExitProcess, -1

	.return:
		ret
	;}
;}
