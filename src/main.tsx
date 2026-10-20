import {FeedbackProvider} from './components/Feedback';
import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { WorkspaceProvider } from './features/workspace/WorkspaceProvider';
import './styles.css';

createRoot(document.getElementById('root')!).render(<React.StrictMode><WorkspaceProvider><FeedbackProvider><App /></FeedbackProvider></WorkspaceProvider></React.StrictMode>);
