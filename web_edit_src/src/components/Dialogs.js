import React from 'react';

// A fullscreen dialog box to communicate major information
export class FullscreenDialog extends React.PureComponent {
  // Return the dialog
  render() {
    return (
      <div className="fullscreenDialog">
        <div className="dialogWindow">
          <div className={`dialogTitle ${this.props.dialogType}`}>{this.props.dialogTitle}</div>
          <div className="dialogMessage">{this.props.dialogMessage}</div>
          {this.props.dialogRetry && <div className="dialogRetry" onClick={this.props.dialogRetry}>Retry ↺</div>}
        </div>
      </div>
    );
  }
}

