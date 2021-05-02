import logoWide from './logo_wide.png';
import { EditMenu } from './components/Menu.js';
import { ViewArea } from './components/DragArea.js';
import './App.css';

function App() {
  return (
    <div className="app">
      <div className="header">
        <img src={logoWide} className="logo" alt="logo" />
        <EditMenu></EditMenu>
      </div>
      <ViewArea></ViewArea>
    </div>
  );
}

export default App;
