import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import App from './App.tsx'
import { AuthProvider } from './auth/AuthProvider.tsx'
import { EntriesProvider } from './entries/EntriesProvider.tsx'

createRoot(document.getElementById('root')!).render(
	<StrictMode>
		<BrowserRouter>
			<AuthProvider>
				<EntriesProvider>
					<App />
				</EntriesProvider>
			</AuthProvider>
		</BrowserRouter>
	</StrictMode>,
)
