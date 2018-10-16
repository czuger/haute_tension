Rails.application.routes.draw do

  devise_for :users
  # resources :adventure_trackers, only: [ :edit, :update ]

  resources :game_logs, only: [ :show ]

  # resources :inventories, only: [ :show, :new, :create, :destroy ]

  resources :adventures, except: [ :edit, :update ] do
    get :reroll
    get :play
    get 'read_choice/:page_id', action: :read_choice, as: :read_choice
    get :roll_dices
    get :die

    resource :heros, only: [ :show, :update ]
    # resources :downloaded_books, only: [:show ]
    resource :items, only: [ :show, :update ]

    resource :fights, only: [ :index, :show, :update, :new, :create ] do
      get :fight_monster
    end
  end

  root 'adventures#index'
end
