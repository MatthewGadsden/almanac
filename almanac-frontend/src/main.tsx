import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import App from './App.tsx'
import { AuthProvider } from './auth/AuthProvider.tsx'
import { ItemsProvider } from './state/ItemsProvider.tsx'

createRoot(document.getElementById('root')!).render(
	<StrictMode>
		<BrowserRouter>
			<AuthProvider>
				<ItemsProvider>
					<App />
				</ItemsProvider>
			</AuthProvider>
		</BrowserRouter>
	</StrictMode>,
)
