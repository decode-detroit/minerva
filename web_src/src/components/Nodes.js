import React from 'react';

// A receive Node element
export class ReceiveNode extends React.PureComponent {  
  // Render the completed node
  render() {
    return (
      <div className={`node ${this.props.type}`} onMouseDown={this.props.onMouseDown}></div>
    );
  }
}

// A send Node element
export class SendNode extends React.PureComponent {
  // Render the completed node
  render() {
    return (
      <div className={`node ${this.props.type}`} onMouseDown={this.props.onMouseDown}></div>
    );
  }
}