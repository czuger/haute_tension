class AdventuresController < ApplicationController
  before_action :set_adventure, only: [:show, :destroy]

  # GET /aventures
  # GET /adventures.json
  def index
    @adventures = Adventure.all
  end

  # GET /adventures/1
  # GET /adventures/1.json
  def show
  end

  def play
    @adventure = Adventure.find(params[:adventure_id])
    @page = @adventure.page
    # to_i.to_s avoid injections
    @page.text.each{ |text| text.gsub!( 'CHANGE_ADVENTURE_ID', params[:adventure_id].to_i.to_s ) }
  end

  def die
  end

  def read_choice
    @adventure = Adventure.find(params[:adventure_id])
    @adventure.page_id = params[:page_id]
    ActiveRecord::Base.transaction do
      @adventure.game_logs.create!( page_id: @adventure.page_id, log_type: GameLog::JOURNEY )
      @adventure.save!
    end
    redirect_to adventure_book_url( @adventure.id, @adventure.page_id )
  end

  # GET /adventures/new
  def new
    @adventure = Adventure.new
    @books = Book.all.order( :name )
  end

  # POST /adventures
  # POST /adventures.json
  def create
    @adventure = Adventure.new(adventure_params)

    book = Book.find(adventure_params[ :book_id ] )
    @adventure.parsed_section = book.first_parsed_section
    roll_adventure

    respond_to do |format|
      if @adventure.save
        @adventure.game_logs.create!( parsed_section_id: @adventure.current_parsed_section_id, log_type: GameLog::JOURNEY )
        format.html { redirect_to @adventure, notice: 'Aventure was successfully created.' }
        format.json { render :show, status: :created, location: @adventure }
      else
        format.html { render :new }
        format.json { render json: @adventure.errors, status: :unprocessable_entity }
      end
    end
  end

  def reroll
    @adventure = Adventure.find(params[:adventure_id])
    roll_adventure
    @adventure.save!
    redirect_to @adventure
  end

  def roll_dices
    @adventure = Adventure.find(params[:adventure_id])
    @dices = Hazard.s2d6
  end

  # DELETE /adventures/1
  # DELETE /adventures/1.json
  def destroy
    @adventure.destroy
    respond_to do |format|
      format.html { redirect_to adventures_url, notice: 'Aventure was successfully destroyed.' }
      format.json { head :no_content }
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_adventure
      @adventure = Adventure.find(params[:id])
    end

    # Never trust parameters from the scary internet, only allow the white list through.
    def adventure_params
      params.require(:adventure).permit(:downloaded_book_id, :adventure_id, :page_id, :hp, :gold, :gourdes, :gourdes_remplies, :rations, :charisme, :fight )
    end

    def roll_adventure
      @adventure.hp = 18 + Hazard.r2d6
      @adventure.hp_max = @adventure.hp
      @adventure.strength = 6 + Hazard.r2d6
      @adventure.strength = 19 if @adventure.strength == 17
      @adventure.strength = 20 if @adventure.strength == 18
      @adventure.strength_max = @adventure.strength
      @adventure.gold = Hazard.r4d6
    end
end
