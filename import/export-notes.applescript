-- export-notes.applescript
-- Dumps notes from a target folder to a delimited UTF-8 corpus file.
-- Edit `targetFolder` / `parentFolder` below, then run:
--   osascript import/export-notes.applescript
-- Output: ~/kammerz-import/corpus/notes-export.txt
--
-- `parentFolder` handles nested folders (e.g. "Photo Log" inside "Hobbies").
-- Set parentFolder to "" if the target is top-level. Set targetFolder to ""
-- to export every note in the account.

property targetFolder : "Photo Log"
property parentFolder : "Hobbies"

on run
	set outPath to (POSIX path of (path to home folder)) & "kammerz-import/corpus/notes-export.txt"
	set outFile to POSIX file outPath
	set fileRef to open for access outFile with write permission
	-- Guarantee the file handle is released even if a Notes call throws mid-export;
	-- otherwise a leaked handle makes the next run fail with "file is busy".
	try
		set eof of fileRef to 0
		set noteCount to 0
		tell application "Notes"
		set theNotes to my collectNotes()
		repeat with n in theNotes
			set noteCount to noteCount + 1
			set rec to "@@@NOTE@@@" & linefeed
			set rec to rec & "TITLE: " & (name of n) & linefeed
			try
				set rec to rec & "FOLDER: " & (name of container of n) & linefeed
			on error
				set rec to rec & "FOLDER: (none)" & linefeed
			end try
			set rec to rec & "CREATED: " & ((creation date of n) as string) & linefeed
			set rec to rec & "MODIFIED: " & ((modification date of n) as string) & linefeed
			set rec to rec & "@@@BODY@@@" & linefeed
			set rec to rec & (body of n) & linefeed
			set rec to rec & "@@@ENDNOTE@@@" & linefeed
			my appendText(fileRef, rec)
		end repeat
		end tell
		close access fileRef
		return "Exported " & noteCount & " notes to " & outPath
	on error errMsg number errNum
		close access fileRef
		error errMsg number errNum
	end try
end run

-- Resolve the target notes, robust to nested folders.
on collectNotes()
	tell application "Notes"
		if targetFolder is "" then return every note
		-- 1) explicit nested path (most reliable for subfolders)
		if parentFolder is not "" then
			try
				return notes of folder targetFolder of folder parentFolder
			end try
		end if
		-- 2) flat folder reference
		try
			return notes of folder targetFolder
		end try
		-- 3) fallback: scan every note, match by immediate container name
		set acc to {}
		repeat with n in (every note)
			try
				if (name of container of n) is targetFolder then set end of acc to n
			end try
		end repeat
		return acc
	end tell
end collectNotes

on appendText(fileRef, t)
	write t to fileRef as «class utf8»
end appendText
