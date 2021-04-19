import React from 'react';

// A menu for the edit items
export class EditMenu extends React.PureComponent {  
  renderSquare(i) {
    return (
      <p>Menu</p>
    );
  }

  // Render the board with all nine squares
  render() {
    return (
      <div className="menu-item">
        {this.renderSquare(0)}
      </div>
    );
  }
}
