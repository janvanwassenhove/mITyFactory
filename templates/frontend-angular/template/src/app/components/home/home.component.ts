import { Component, inject } from '@angular/core';
import { CounterComponent } from '../counter/counter.component';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [CounterComponent],
  template: `
    <div class="home">
      <h1>Welcome to {{project_name}}</h1>
      <p>Powered by Angular 17+ with standalone components</p>
      <app-counter></app-counter>
    </div>
  `,
  styles: [`
    .home {
      text-align: center;
    }
    h1 {
      color: #dd0031;
      font-size: 2.5rem;
      margin-bottom: 1rem;
    }
    p {
      color: #666;
      margin-bottom: 2rem;
    }
  `]
})
export class HomeComponent {}
