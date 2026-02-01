import { Component, signal, computed } from '@angular/core';

@Component({
  selector: 'app-counter',
  standalone: true,
  template: `
    <div class="counter">
      <h2>Counter: {{ count() }}</h2>
      <p>Double: {{ doubleCount() }}</p>
      <div class="buttons">
        <button (click)="decrement()">-</button>
        <button (click)="increment()">+</button>
      </div>
    </div>
  `,
  styles: [`
    .counter {
      margin-top: 2rem;
      padding: 1.5rem;
      background: #f5f5f5;
      border-radius: 8px;
      display: inline-block;
    }
    h2 {
      margin: 0 0 0.5rem;
      color: #333;
    }
    p {
      color: #666;
      margin-bottom: 1rem;
    }
    .buttons {
      display: flex;
      gap: 0.5rem;
      justify-content: center;
    }
    button {
      padding: 0.5rem 1.5rem;
      font-size: 1.25rem;
      border: none;
      border-radius: 4px;
      background: #dd0031;
      color: white;
    }
    button:hover {
      background: #c3002f;
    }
  `]
})
export class CounterComponent {
  count = signal(0);
  doubleCount = computed(() => this.count() * 2);

  increment() {
    this.count.update(c => c + 1);
  }

  decrement() {
    this.count.update(c => c - 1);
  }
}
