export const SLOTH_URL = 'https://raw.githubusercontent.com/MatthewGadsden/MatthewGadsden/main/images/sloth.png'
export const REPO_URL = 'https://github.com/MatthewGadsden/almanac'

export const TMDB_URL = 'https://www.themoviedb.org'
/** The notice TMDB's terms ask apps using its API to show. */
export const TMDB_NOTICE = 'This product uses the TMDB API but is not endorsed or certified by TMDB.'

export function tmdbMovieUrl(tmdbId: number): string {
	return `${TMDB_URL}/movie/${tmdbId}`
}
