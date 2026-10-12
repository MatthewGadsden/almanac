import { useEffect, useRef } from 'react'
import { renderNoteHtml } from '../noteHtml'
import noteBodyCss from '../styling/note-body.css?inline'

/**
 * Renders a note's HTML inside a shadow root. The base typography lives in note-body.css;
 * the note's own <style> blocks come after it, so they override it, and neither leaks out
 * into the rest of the app. Theme custom properties (var(--text) etc.) still inherit in.
 */
export function NoteBody({ body, className }: { body: string; className?: string }) {
	const hostRef = useRef<HTMLDivElement>(null)

	useEffect(() => {
		const host = hostRef.current
		if (!host) {
			return
		}

		const root = host.shadowRoot ?? host.attachShadow({ mode: 'open' })
		root.innerHTML = `<style>${noteBodyCss}</style><div class="note-root">${renderNoteHtml(body)}</div>`
	}, [body])

	return <div className={className} ref={hostRef} />
}
