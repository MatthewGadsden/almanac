import DOMPurify from 'dompurify'

/**
 * Note bodies are HTML. Bodies written before HTML support (or anything without a tag)
 * are treated as plain text, so blank lines still separate paragraphs.
 */
export function looksLikeHtml(body: string): boolean {
	return /<\/?[a-z][\s\S]*>/i.test(body)
}

function escapeHtml(text: string): string {
	return text
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&#39;')
}

/**
 * Sanitised HTML for a note body. `<style>` blocks and `style` attributes are kept so a note
 * can be styled freely (it renders inside a shadow root, which keeps that CSS to the note).
 * Scripts, event handler attributes and `javascript:` URLs are removed.
 */
export function renderNoteHtml(body: string): string {
	if (!looksLikeHtml(body)) {
		return body
			.split(/\n\s*\n/)
			.filter((paragraph) => paragraph.trim())
			.map((paragraph) => `<p class="plain">${escapeHtml(paragraph)}</p>`)
			.join('')
	}

	// FORCE_BODY keeps a leading <style> from being hoisted into <head> and dropped
	return DOMPurify.sanitize(body, { ADD_TAGS: ['style'], FORCE_BODY: true })
}

const BLOCK_ELEMENTS = 'p, div, h1, h2, h3, h4, h5, h6, li, blockquote, pre, figcaption, tr, td, th, br, hr'

/** The readable text of a note body, for excerpts. */
export function plainText(body: string): string {
	if (!looksLikeHtml(body)) {
		return body
	}

	// DOMParser documents are inert: no scripts run and no images load
	const doc = new DOMParser().parseFromString(body, 'text/html')
	doc.querySelectorAll('style, script').forEach((element) => element.remove())
	// textContent runs blocks together ("end.Next heading"), so separate them first
	doc.body.querySelectorAll(BLOCK_ELEMENTS).forEach((element) => element.append(' '))
	return doc.body.textContent ?? ''
}
