import { bootstrapApplication } from '@angular/platform-browser';
import { appConfig } from './app/app.config';
import { AppComponent } from './app/app.component';
import { applySkemaStyle } from './style/skema-style';

applySkemaStyle(document.documentElement);

bootstrapApplication(AppComponent, appConfig)
  .catch((err) => console.error(err));
